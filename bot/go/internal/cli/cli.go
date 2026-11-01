// Package cli is bot/harness/__main__.py.
//
// The command line: `harness <command>` from the `bot/` directory.
//
// The workflows call `peek`, `quiet`, `plan`, `work`, `deliver`, `event` and `sweep`. An operator
// calls `status`, `providers`, `halt`, `start`, `dispatch`, `doctor`, `setup` and `forget`, with a
// token in BOT_GITHUB_TOKEN, GITHUB_TOKEN or GH_TOKEN, or a logged-in `gh`.
//
// Two more commands stand for the workflows' other ways into the harness, and are not in the
// help, which stays `harness`'s: `titles shipped …` is `python3 -m harness.titles …`
// (patches-ship.yml), and `pr-title --title=…` is the inline check of pr-title.yml.
package cli

import (
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"runtime/debug"
	"strconv"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/cardsets"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/controller"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/dashboard"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/deliver"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/disk"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/events"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/identity"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/logins"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/quiet"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/redact"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/stats"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/status"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/sweep"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/titles"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/triage"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/trust"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/wake"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/work"
)

// ---------------------------------------------------------------------------- test seams

// Stdout and Stderr are where `print` writes (sys.stdout and sys.stderr); variables so the tests
// can read them, as `contextlib.redirect_stdout` does.
var (
	Stdout io.Writer = os.Stdout
	Stderr io.Writer = os.Stderr
)

// Sleep is time.sleep; a variable so the tests can replace it.
var Sleep = func(seconds float64) { time.Sleep(time.Duration(seconds * float64(time.Second))) }

var started = time.Now()

// Monotonic is time.monotonic; a variable so the tests can replace it.
var Monotonic = func() float64 { return time.Since(started).Seconds() }

// SubprocessRun is subprocess.run as this module calls it; a variable so the tests can replace
// it.
var SubprocessRun = subprocessRun

// Sweep is sweep.sweep as this module calls it; a variable so the tests can replace it.
var Sweep = sweep.Sweep

// GithubEnv is config.github_env as this module calls it; a variable so the tests can replace
// it.
var GithubEnv = config.GithubEnv

func say(line string)    { fmt.Fprintln(Stdout, line) }
func sayErr(line string) { fmt.Fprintln(Stderr, line) }

// ---------------------------------------------------------------------------- arguments

// Args is the argparse.Namespace a command reads: every argument of every command, the zero
// value where the command has none (the Python's `getattr(args, name, False)`).
type Args struct {
	Command string

	Out     string  // --out (plan, work, plan-fetch, deliver)
	Force   bool    // --force (plan, peek, stats, dispatch)
	QuietOK *string // --quiet-ok (plan): None unless given
	Item    string  // --item (plan, peek, dispatch)
	Mode    string  // --mode (plan, peek, dispatch)

	Wait bool   // --wait (wake)
	Now  bool   // --now (wake)
	List bool   // --list (wake)
	Ping string // --ping (wake)

	Plan    string // --plan (work, deliver, doctor)
	WorkDir string // --work-dir (work)

	Ref   string // --ref (plan-fetch)
	Check bool   // --check (plan-fetch)

	Action   string // --action (deliver)
	Number   string // --number (deliver); the number (forget)
	Provider string // --provider (deliver)

	Name    string // --name (event)
	Payload string // --payload (event, triage)

	Every        int  // --every (dashboard)
	RunnersEvery int  // --runners-every (dashboard)
	ForSeconds   int  // --for (dashboard)
	Sweep        bool // --sweep (dashboard)
	Stats        bool // --stats (dashboard)
	Companions   bool // --companions (dashboard, stats)
	Controller   bool // --controller (dashboard)
	SectionOnly  bool // --section-only (dashboard, stats)

	Reason []string // the reason (halt)

	Offline      bool // --offline (providers, window)
	Work         bool // --work (doctor)
	RepoSettings bool // --repo-settings (setup)
}

// argsOf is the namespace as Args.
func argsOf(ns map[string]any) *Args {
	s := func(k string) string { v, _ := ns[k].(string); return v }
	b := func(k string) bool { v, _ := ns[k].(bool); return v }
	n := func(k string) int { v, _ := ns[k].(int); return v }
	a := &Args{
		Command: s("command"), Out: s("out"), Force: b("force"), Item: s("item"), Mode: s("mode"),
		Wait: b("wait"), Now: b("now"), List: b("list"), Ping: s("ping"),
		Plan: s("plan"), WorkDir: s("work_dir"), Ref: s("ref"), Check: b("check"),
		Action: s("action"), Number: s("number"), Provider: s("provider"),
		Name: s("name"), Payload: s("payload"),
		Every: n("every"), RunnersEvery: n("runners_every"), ForSeconds: n("for_seconds"),
		Sweep: b("sweep"), Stats: b("stats"), Companions: b("companions"), Controller: b("controller"),
		SectionOnly: b("section_only"), Offline: b("offline"), Work: b("work"),
		RepoSettings: b("repo_settings"),
	}
	if q, ok := ns["quiet_ok"].(string); ok {
		a.QuietOK = &q
	}
	if r, ok := ns["reason"].([]string); ok {
		a.Reason = r
	}
	return a
}

// ParseArgs is parse_args: the arguments, or the exit argparse takes (its help on stdout and 0,
// or the usage and an error on stderr and 2) and false.
func (p *ArgumentParser) ParseArgs(argv []string) (*Args, int, bool) {
	ns, exit := p.parse(argv)
	if exit != nil {
		io.WriteString(Stdout, exit.stdout)
		io.WriteString(Stderr, exit.stderr)
		return nil, exit.code, false
	}
	return argsOf(ns), 0, true
}

// ---------------------------------------------------------------------------- helpers

// localToken is _local_token: a logged-in `gh`'s token, for an operator running this by hand.
func localToken() string {
	gh, err := exec.LookPath("gh")
	if err != nil {
		return ""
	}
	proc, err := SubprocessRun([]string{gh, "auth", "token"}, RunOptions{})
	herr.Must(err)
	if proc.Returncode == 0 {
		return strings.TrimSpace(proc.Stdout)
	}
	return ""
}

// Ctx is _ctx; a variable so the tests can replace it.
var Ctx = ctxFor

func ctxFor(cfg *config.Config, write bool /* = True */) *hctx.Context {
	if write && cfg.WriteToken() != "" {
		return build(cfg, nil)
	}
	token := cfg.WriteToken()
	if !write {
		token = cfg.ActionsToken
		if token == "" {
			token = cfg.BotToken
		}
	}
	if token == "" {
		token = localToken()
	}
	return build(cfg, &token)
}

// build is `context_mod.build(cfg[, token=token])`, where a nil `token` is Python's None (the
// config's write token, with the Actions token as its fallback and for Actions calls).
//
// hctx.Build takes the token as a string, "" for None.
func build(cfg *config.Config, token *string) *hctx.Context {
	t := ""
	if token != nil {
		t = *token
	}
	return hctx.Build(cfg, t, nil, nil)
}

// Output is _output; a variable so the tests can replace it.
var Output = output

func output(pairs *pyjson.Object) {
	path := GithubEnv()["GITHUB_OUTPUT"]
	if path == "" {
		return
	}
	handle, err := os.OpenFile(path, os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0o666)
	herr.Must(err)
	defer handle.Close()
	pairs.Items(func(key string, value any) bool {
		_, err := fmt.Fprintf(handle, "%s=%s\n", key, py.Str(value))
		herr.Must(err)
		return true
	})
}

// Summary is _summary; a variable so the tests can replace it.
var Summary = summary

func summary(text string) {
	path := GithubEnv()["GITHUB_STEP_SUMMARY"]
	if path != "" {
		appendText(path, redact.Redact(text, nil)+"\n")
	}
}

// appendText is `open(path, "a", encoding="utf-8").write(text)`; an OSError goes up.
func appendText(path, text string) {
	handle, err := os.OpenFile(path, os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0o666)
	herr.Must(err)
	defer handle.Close()
	_, err = io.WriteString(handle, text)
	herr.Must(err)
}

// bullets is _bullets: `lines` as a Markdown list after a blank line, or nothing when there are
// none.
func bullets(lines []string) string {
	if len(lines) == 0 {
		return ""
	}
	var b strings.Builder
	b.WriteString("\n")
	for _, line := range lines {
		b.WriteString("- " + line + "\n")
	}
	return b.String()
}

// Dump is _dump.
func Dump(obj any) {
	say(redact.Redact(pyjson.DumpsIndent(obj, 2), nil))
}

// itemArg is `int(args.item) if str(args.item or "").strip().lstrip("#").isdigit() else None`:
// the test strips a `#` that int() then refuses, so `--item '#5'` raises its ValueError, as in the
// Python.
func itemArg(item string) *int {
	if !py.IsDecimal(strings.TrimLeft(strings.TrimSpace(item), "#")) {
		return nil
	}
	n := py.MustInt(item)
	return &n
}

// ---------------------------------------------------------------------------- workflow commands

// LockTries and LockWait (seconds) are LOCK_TRIES and LOCK_WAIT: how many times, and how far
// apart in seconds, a plan job asks for the planning lock the status loop may hold (#1455): about
// five minutes, long enough for a loop starting every lane at once, and well inside the plan
// job's 15-minute timeout.
var (
	LockTries         = 30
	LockWait  float64 = 10.0
)

// CmdPlan is cmd_plan.
func CmdPlan(cfg *config.Config, args *Args) int {
	ctx := Ctx(cfg, true)
	item := itemArg(args.Item)
	runID := cfg.RunID
	if runID == "" {
		runID = "by hand"
	}
	owner := "run " + runID
	locked := !cfg.Controller || planningLock(ctx, owner)
	var planned *pyjson.Object
	func() {
		defer func() {
			if locked && cfg.Controller {
				plan.DropPlanningLock(ctx, owner)
			}
		}()
		if locked {
			// `quiet_ok=None` (a run by hand) restricts nothing: plan.make reads it as ANY_QUIET.
			planned = plan.Make(ctx, args.Force, item, args.Mode, args.QuietOK)
		} else {
			planned = plan.Nothing("the status loop is planning; it starts the runs (#1455)")
		}
	}()
	mkdirParent(args.Out)
	writeText(args.Out, pyjson.DumpsIndent(planned, 2)+"\n")
	// The provider, its CLI and the name of its secret steer the model job: which CLI to install
	// and which one secret to hand it. They come from providers.json, never from the model.
	Output(pyjson.New("action", py.Item(planned, "action"), "number", py.Or(planned.Get("number"), ""),
		"provider", py.Or(planned.Get("provider"), ""), "cli", py.Or(planned.Get("cli"), ""),
		"secret", py.Or(planned.Get("secret"), ""),
		"shared", lower(planned.Truthy("shared")),
		"runs_on", py.Or(planned.Get("runs_on"), "ubuntu-latest")))
	action := py.Str(py.Item(planned, "action"))
	what := action
	if action != "none" && planned.Truthy("number") {
		what = fmt.Sprintf("%s #%s", action, py.Str(planned.Get("number")))
	}
	if planned.Truthy("provider") {
		what += " on " + py.Str(planned.Get("provider"))
	}
	if planned.Truthy("priority") {
		what += ", priority tier " + py.Str(planned.Get("priority"))
	}
	if planned.Truthy("difficulty") {
		what += ", difficulty " + py.Str(planned.Get("difficulty"))
	}
	if planned.Truthy("assignment") {
		what += " (" + py.Str(planned.Get("assignment")) + ")"
	}
	var notes []string
	for _, k := range []string{"routing", "skipped"} {
		for _, line := range pyjson.List(py.Or(planned.Get(k), []any{})) {
			notes = append(notes, py.Str(line))
		}
	}
	reason := py.Str(planned.GetOr("reason", ""))
	Summary(fmt.Sprintf("### Plan: %s\n\n%s\n%s", what, reason, bullets(notes)))
	say(strings.TrimSpace(fmt.Sprintf("plan: %s %s", what, reason)))
	for _, line := range notes {
		say("plan: " + line)
	}
	return 0
}

// planningLock is _planning_lock.
func planningLock(ctx *hctx.Context, owner string) bool {
	for attempt := 0; attempt < LockTries; attempt++ {
		if plan.TakePlanningLock(ctx, owner) {
			return true
		}
		if attempt < LockTries-1 {
			Sleep(LockWait)
		}
	}
	return false
}

// CmdPeek is cmd_peek.
func CmdPeek(cfg *config.Config, args *Args) int {
	ctx := Ctx(cfg, false)
	item := itemArg(args.Item)
	if cfg.Controller && item == nil && !args.Force && args.Mode == "auto" && plan.LoopAlive(ctx) {
		// The status loop plans and starts the runs while it goes (#1455): a run started by the
		// schedule, or by a chain from before the loop, stops here. One a person started for an
		// item, a mode or by force plans itself, under the planning lock.
		Output(pyjson.New("work", "false", "quiet_check", "false", "quiet_provider", "", "quiet_secret", "",
			"fallback", "false"))
		Summary("### Peek: the status loop plans\n\nbot-status's loop is going, and it plans the " +
			"work and starts the runs itself (#1455).")
		say("peek: nothing: the status loop plans and starts the runs")
		return 0
	}
	look := plan.Peek_(ctx, args.Force, item, args.Mode)
	check := look.Work && look.QuietProvider != ""
	Output(pyjson.New("work", lower(look.Work), "quiet_check", lower(check),
		"quiet_provider", look.QuietProvider, "quiet_secret", look.QuietSecret,
		"fallback", lower(look.Fallback)))
	then := ""
	if check {
		then = fmt.Sprintf(" First, `%s` must be quiet", look.QuietProvider)
		if look.Fallback {
			then += " (other work can go ahead if it is not)."
		} else {
			then += "."
		}
	}
	heading, verdict := "nothing to do", "nothing"
	if look.Work {
		heading, verdict = "work", "work"
	}
	Summary(fmt.Sprintf("### Peek: %s\n\n%s.%s\n%s", heading, look.Reason, then, bullets(look.Skipped)))
	say(fmt.Sprintf("peek: %s: %s.%s", verdict, look.Reason, then))
	for _, line := range look.Skipped {
		say("peek: " + line)
	}
	return 0
}

// CmdQuiet is cmd_quiet.
func CmdQuiet(cfg *config.Config, args *Args) int {
	ctx := Ctx(cfg, false)
	token := cfg.ActionsToken
	if token == "" {
		token = cfg.BotToken
	}
	verdict := quiet.WaitForQuiet(
		cfg.Quiet,
		func() *pyjson.Object { return runner.PingUsage(cfg.ClaudeBin(), cfg.Quiet.PingModel, nil, "", "") },
		func(t1, t2 time.Time) string {
			return quiet.PartnerSpending(cfg.Quiet.Partners,
				func(repo string) quiet.RunReader { return quiet.NewPartnerReader(repo, token, nil) }, t1, t2)
		},
		ctx.Now,
		Sleep,
	)
	Output(pyjson.New("quiet", lower(verdict.Quiet)))
	excused := ""
	if verdict.ExcusedBy != "" {
		excused = fmt.Sprintf(" (the rise was %s)", verdict.ExcusedBy)
	}
	yes := "no"
	if verdict.Quiet {
		yes = "yes"
	}
	Summary(fmt.Sprintf("### Quiet: %s\n\n%s%s.\n", yes, verdict.Reason, excused))
	Dump(verdict.ToDict())
	return 0
}

// CmdWake is cmd_wake: the wake (`wake.py`): `--wait` sleeps until the wake time, `--list` prints
// this bot's subscriptions to wake as JSON on its last line, and `--ping` wakes one with the job's
// own secret.
func CmdWake(cfg *config.Config, args *Args) int {
	if args.Ping != "" {
		ok, line := wake.Wake(cfg, args.Ping)
		say(line)
		Summary(line)
		if ok {
			return 0
		}
		return 1
	}
	ctx := Ctx(cfg, false)
	now := ctx.Now()
	home := identity.Home_
	if args.List {
		found, notes := wake.ToWake(ctx, home)
		say(pyjson.Dumps(found))
		for _, line := range notes {
			sayErr("wake: " + line)
		}
		Summary(fmt.Sprintf("### Wake (%s)\n\n%d to wake.%s", home, len(found), bullets(notes)))
		return 0
	}
	var at time.Time
	if !args.Now {
		at = wake.WakeTime(now)
	}
	reason := ""
	if !args.Now && at.IsZero() {
		reason = fmt.Sprintf("outside the hours around the wake times (%s ", hhmm(wake.WakeAt)) +
			fmt.Sprintf("%s and every %s around it)", wake.Zone, timedelta(wake.Every))
	} else if !at.IsZero() && wake.AlreadyWoken(ctx.GH.ListRuns(wake.Workflow, "success", 10, false), now) {
		reason = fmt.Sprintf("an earlier run already woke the sessions of %s %s", hhmm(at), wake.Zone)
	}
	if reason != "" {
		say("wake: nothing to do: " + reason)
		Output(pyjson.New("go", "false"))
		return 0
	}
	if !at.IsZero() && at.After(now) {
		say(fmt.Sprintf("wake: sleeping %s until %s %s", timedelta(at.Sub(now)), hhmm(at), wake.Zone))
		Sleep(at.Sub(now).Seconds())
	}
	Output(pyjson.New("go", "true"))
	return 0
}

// MakeProbe is make_probe: between a run's steps: whether it must stop. `last_hour` gives the
// reset of the 5-hour session whose last hour the run took (`work.Worker.last_hour`), whose cap
// stays lifted.
//
// The probe returns nil (None) to go on, or the reason and its kind (`work.Probe`).
func MakeProbe(ctx *hctx.Context, number *int, provider *providers.Provider /* = None */, lastHour func() time.Time /* = lambda: None */) work.Probe {
	if provider == nil {
		provider = ctx.Cfg.Pool.Ordered()[0]
	}
	if lastHour == nil {
		lastHour = func() time.Time { return time.Time{} }
	}

	closed := func(thread int) bool {
		var issue *pyjson.Object
		if _, ok := herr.Catch[*herr.GitHubError](func() { issue = ctx.GH.GetIssue(thread) }); ok {
			return false // GitHub could not say: carry on, as before this check
		}
		return issue.Get("state") == "closed"
	}

	probe := func(lastUsage *pyjson.Object) *[2]string {
		if ctx.RepoHalted() {
			return &[2]string{fmt.Sprintf("halted by %s on main", config.HaltPath), "halt"}
		}
		st := ctx.Store.Load()
		if st.Truthy("halted") {
			return &[2]string{fmt.Sprintf("halted by %s halt", config.Slash), "halt"}
		}
		held := providers.Suspension(st, provider.ID)
		if held != nil {
			by := ""
			if held.Truthy("by") {
				by = " by @" + py.Str(py.Item(held, "by"))
			}
			return &[2]string{fmt.Sprintf("`%s` was suspended%s", provider.ID, by), "suspend"}
		}
		if number != nil {
			record := st.Obj("items").Obj(strconv.Itoa(*number))
			if record.Truthy("stop_requested") {
				return &[2]string{"stopped by @" + py.Str(record.GetOr("stopped_by", "someone")), "stop"}
			}
			if closed(*number) {
				// Deliver drops a closed thread's work anyway, so stop spending on it now.
				return &[2]string{fmt.Sprintf("#%d was closed", *number), "stop"}
			}
		}
		providers.NoteUsage(st, provider.ID, lastUsage, "", ctx.Now(), 0.0, nil, nil)
		refusal := providers.Refusal(provider, providers.PeekRecord(st, provider.ID), ctx.Now(),
			ctx.Cfg.Timezone, false, lastHour())
		if refusal != "" {
			return &[2]string{fmt.Sprintf("`%s` stops: %s", provider.ID, refusal), "usage"}
		}
		return nil
	}
	return func(lastUsage *pyjson.Object) (string, string, bool) {
		if found := probe(lastUsage); found != nil {
			return found[0], found[1], true
		}
		return "", "", false
	}
}

// usagePoller is a runner that `polls_usage` (agy, Muse): its `read_usage` reads the
// subscription's usage with no model call.
type usagePoller interface {
	PollsUsage() bool
	ReadUsage(model string) *pyjson.Object
}

// CmdWork is cmd_work.
func CmdWork(cfg *config.Config, args *Args) int {
	planned := object(readJSON(args.Plan))
	ctx := Ctx(cfg, false)
	number := planned.Get("number")
	out := args.Out
	provider := cfg.Pool.Get(pyjson.Str(planned.Get("provider")))
	if provider == nil {
		provider = cfg.Pool.Ordered()[0]
	}
	var login *logins.Login
	secret := ""
	if cfg.Backend != "fake" {
		if provider.Login == "secret" {
			secret = cfg.SecretFor(provider.Secret)
		}
		if secret != "" || provider.CLI != "claude" || provider.Login == "machine" {
			exc, failed := herr.Catch[*herr.LoginError](func() {
				login = logins.Prepare(provider, secret, py.OrStr(planned.Get("vault"), ""),
					filepath.Join(args.WorkDir, ".logins", provider.ID))
			})
			if failed {
				mkdirAll(out)
				result := pyjson.New("version", 1, "action", planned.Get("action"), "number", number,
					"status", "infra", "reason", redact.Redact(exc.Error(), nil), "provider", provider.ID)
				writeText(filepath.Join(out, "result.json"), pyjson.DumpsIndent(result, 2)+"\n")
				say("work: infra: " + redact.Redact(exc.Error(), nil))
				return 0
			}
		}
	}
	keepLogin := func() {
		// After every model call: seal a login the CLI refreshed (and redact its new tokens from
		// then on), so even a job killed later hands it on to the next run.
		if login == nil {
			return
		}
		var sealed string
		if exc, ok := catchIf(func() { sealed = logins.Seal(login) },
			func(err error) bool { return isValueError(err) || herr.IsOSError(err) }); ok {
			say("work: could not seal the refreshed login: " + redact.Redact(herr.Str(exc), nil))
			return
		}
		if sealed != "" {
			mkdirAll(out)
			writeText(filepath.Join(out, "vault.enc"), sealed+"\n")
		}
	}

	run := runner.GetRunner(cfg, provider, login)
	var item *int
	if pyjson.Truthy(number) {
		n := py.MustInt(number)
		item = &n
	}
	var worker *work.Worker
	worker = work.NewWorker(cfg, planned, run, cfg.Root, args.WorkDir, out,
		MakeProbe(ctx, item, provider, func() time.Time { return worker.LastHour }),
		keepLogin, nil, nil, nil, nil)
	if cfg.Backend != "fake" && provider.Limits.Stops().Len() > 0 {
		// A fresh reading before any model work: the stored one is the last run's, and none at
		// all once its window reset. Claude: one Haiku turn, signed in as this subscription; agy
		// and Muse: their own `/usage`, no model call.
		if provider.CLI == "claude" {
			token := ""
			if provider.Login == "secret" {
				token = secret
			}
			worker.StartUsage = runner.PingUsage(cfg.ClaudeBin(), cfg.Quiet.PingModel, nil, token, "")
		} else if poller, ok := run.(usagePoller); ok && poller.PollsUsage() {
			worker.StartUsage = poller.ReadUsage(provider.Model)
		}
	}
	result := worker.Run()
	keepLogin()
	Summary(fmt.Sprintf("### Work: %s\n\n%s\n", py.Str(result.Get("status")), py.Str(result.GetOr("reason", ""))))
	say(fmt.Sprintf("work: %s: %s", py.Str(result.Get("status")), py.Str(result.GetOr("reason", ""))))
	return 0
}

// CmdPlanFetch is cmd_plan_fetch: a night run the status loop started with its plan (#1455): read
// the plan from the state branch into `--out`. With `--check` (the model job), the runner, CLI,
// secret name and shared flag the job was given (`PLAN_RUNS_ON`, `PLAN_CLI`, `PLAN_SECRET`,
// `PLAN_SHARED`) must be the plan's and its subscription's in providers.json, or the job stops
// before any step has a secret.
func CmdPlanFetch(cfg *config.Config, args *Args) int {
	ctx := Ctx(cfg, false)
	var planned *pyjson.Object
	if exc, ok := catchIf(func() {
		var err error
		if planned, err = controller.Fetch(ctx, args.Ref); err != nil {
			herr.Raise(err)
		}
	}, func(err error) bool {
		_, gh := err.(*herr.GitHubError)
		return gh || isValueError(err)
	}); ok {
		sayErr(redact.Redact("::error::plan-fetch: "+exc.Error(), nil))
		return 2
	}
	if args.Check {
		env := GithubEnv()
		given := map[string]string{"runs_on": env["PLAN_RUNS_ON"], "cli": env["PLAN_CLI"],
			"secret": env["PLAN_SECRET"], "shared": env["PLAN_SHARED"]}
		problems := controller.Check(ctx, planned, given)
		if len(problems) > 0 {
			for _, problem := range problems {
				sayErr(redact.Redact("::error::plan-fetch: "+problem, nil))
			}
			return 2
		}
	}
	mkdirParent(args.Out)
	writeText(args.Out, pyjson.DumpsIndent(planned, 2)+"\n")
	say(fmt.Sprintf("plan-fetch: %s, plan %s", controller.What(planned), args.Ref))
	return 0
}

// CmdDeliver is cmd_deliver.
func CmdDeliver(cfg *config.Config, args *Args) int {
	planned := object(readJSON(args.Plan))
	var action any = args.Action
	if args.Action == "" {
		action = planned.Get("action")
	}
	if action == nil || action == "" || action == "none" {
		say("deliver: nothing was planned")
		return 0
	}
	ctx := Ctx(cfg, true)
	var number *int
	if py.IsDecimal(args.Number) {
		n := py.MustInt(args.Number)
		number = &n
	}
	outcome := deliver.NewDeliverer(ctx, planned, args.Out, cfg.Root, args.Action, number, args.Provider).Run()
	var log []string
	for _, line := range pyjson.List(py.Item(outcome, "log")) {
		log = append(log, py.Str(line))
	}
	items := make([]string, len(log))
	for i, line := range log {
		items[i] = "- " + line
	}
	Summary(fmt.Sprintf("### Deliver: %s\n\n", py.Str(outcome.Get("status"))) + strings.Join(items, "\n"))
	say(fmt.Sprintf("deliver: %s; ", py.Str(outcome.Get("status"))) + strings.Join(log, "; "))
	return 0
}

// CmdEvent is cmd_event.
func CmdEvent(cfg *config.Config, args *Args) int {
	env := GithubEnv()
	name := args.Name
	if name == "" {
		name = env["GITHUB_EVENT_NAME"]
	}
	path := args.Payload
	if path == "" {
		path = env["GITHUB_EVENT_PATH"]
	}
	if name == "" || path == "" {
		sayErr("event: no event name or payload")
		return 2
	}
	payload := object(readJSON(path))
	for _, line := range events.Handle(Ctx(cfg, true), name, payload) {
		say(redact.Redact("event: "+line, nil))
	}
	return 0
}

// ---------------------------------------------------------------------------- operator commands

// CmdSweep is cmd_sweep.
func CmdSweep(cfg *config.Config, args *Args) int {
	notes := Sweep(Ctx(cfg, true))
	if len(notes) == 0 {
		notes = []string{"nothing was left unanswered"}
	}
	for _, line := range notes {
		say(redact.Redact("sweep: "+line, nil))
	}
	items := make([]string, len(notes))
	for i, n := range notes {
		items[i] = "- " + n
	}
	Summary("### Sweep\n\n" + strings.Join(items, "\n"))
	return 0
}

// CmdStatus is cmd_status.
func CmdStatus(cfg *config.Config, args *Args) int {
	say(status.Report(Ctx(cfg, true)))
	return 0
}

// runCreated is _run_created: when this workflow run was created, which is when GitHub fixed its
// list of secrets; None (zero) run by hand, or when it cannot be read.
func runCreated(cfg *config.Config) time.Time {
	if cfg.RunID == "" {
		return time.Time{}
	}
	var at time.Time
	if err := herr.Try(func() { at = clock.ParseISOValue(Ctx(cfg, true).GH.GetRun(cfg.RunID).Get("created_at")) }); err != nil {
		return time.Time{} // not knowing only means trusting the newest record
	}
	return at
}

// current is _current: the loop's config as it stands now. The `bot-status` job reads
// `.harness/` once, when it starts, and runs for five and a half hours, and GitHub fixes its
// secrets when its run is created, hours before that for a run queued behind the last loop; so a
// subscription added or changed since would show as it was. Each tick reads the subscriptions
// again from the default branch, and takes which secrets are set from the newest plan job's
// record when that is newer than this run (`providers.newer_secrets`). What cannot be read stays
// as it was.
//
// None when this checkout's code refuses the default branch's subscriptions: it read its own file
// when it started, so the branch's has changed since in a way only newer code reads (a
// subscription with a secret this checkout does not know, as claude-7's was), and the loop ends so
// that the next one starts on the default branch's code.
func current(cfg *config.Config, since time.Time) *config.Config {
	pool, secrets := cfg.Pool, cfg.Secrets
	refused := false
	if err := herr.Try(func() {
		ctx := Ctx(cfg, true)
		text, _ := ctx.GH.GetFile(providers.ProvidersPath, cfg.DefaultBranch)
		if text != "" {
			raw := loads(text)
			if exc, ok := herr.Catch[*herr.ConfigError](func() { pool = providers.Parse(raw) }); ok {
				say(redact.Redact(fmt.Sprintf("::notice::this checkout cannot read the subscriptions on "+
					"%s (%s): the next loop starts on its code", cfg.DefaultBranch, exc.Error()), nil))
				refused = true
				return
			}
		}
		secrets = providers.NewerSecrets(cfg.Secrets, ctx.Store.Load(), since)
	}); err != nil {
		// one bad tick must not end the loop
		say(redact.Redact("::warning::kept the subscriptions and secrets as they were: "+herr.Str(err), nil))
	}
	if refused {
		return nil
	}
	fresh := *cfg
	fresh.Pool, fresh.Secrets = pool, secrets
	return &fresh
}

// Companion is _companion: run `harness <argv>` as another bot (`config.OTHERS`, #60):
// its home, its token. One process is one bot, since the labels and branches are fixed when it
// starts. Here the other process is this binary, run from `bot/` as the Python was; the error is
// Python's OSError or TimeoutExpired.
func Companion(other *identity.Other, argv ...string) (*Completed, error) {
	exe, err := os.Executable()
	if err != nil {
		return nil, err
	}
	dir := ""
	if root := config.RepoRoot; root != "" {
		dir = filepath.Join(root, "bot")
	}
	return SubprocessRun(append([]string{exe}, argv...), RunOptions{
		Dir: dir, Env: config.CompanionEnv(other, nil),
		Timeout: time.Duration(CompanionTimeout) * time.Second,
	})
}

// CompanionTimeout is COMPANION_TIMEOUT: how long a companion's sweep or section may take before
// the tick goes on without it, in seconds.
var CompanionTimeout = 300

// companions is _companions: each other bot's section (Squishy's, #60), after its sweep when the
// loop sweeps: its own process draws it from its own state and labels, `argv` saying which (its
// section of the status issue, or with `("stats", "--section-only")` of the statistics issue). A
// bot that cannot be run is a warning, and the issue goes on without its section.
func companions(cfg *config.Config, doSweep bool, argv []string /* = ("dashboard", "--section-only") */, what string /* = "section" */) []string {
	var sections []string
	for _, other := range config.Others {
		if other.Home == "" {
			continue
		}
		if doSweep && other.TokenEnv != "" && config.CompanionEnv(other, nil)["BOT_GITHUB_TOKEN"] != "" {
			done, err := Companion(other, "sweep")
			if err != nil {
				say(redact.Redact(fmt.Sprintf("::warning::%s's sweep did not run: %s", other.Name, herr.Str(err)), nil))
			} else {
				for _, line := range py.Splitlines(done.Stdout) {
					say(redact.Redact(fmt.Sprintf("%s: %s", other.Name, line), nil))
				}
				if done.Returncode != 0 {
					say(redact.Redact(fmt.Sprintf("::warning::%s's sweep failed: %s", other.Name,
						py.SliceFrom(done.Stderr, -500)), nil))
				}
			}
		}
		drawn, err := Companion(other, argv...)
		if err != nil {
			say(redact.Redact(fmt.Sprintf("::warning::%s's %s was not drawn: %s", other.Name, what, herr.Str(err)), nil))
			continue
		}
		if drawn.Returncode == 0 && strings.TrimSpace(drawn.Stdout) != "" {
			sections = append(sections, drawn.Stdout)
		} else {
			say(redact.Redact(fmt.Sprintf("::warning::%s's %s was not drawn: %s", other.Name, what,
				py.SliceFrom(drawn.Stderr, -500)), nil))
		}
	}
	return sections
}

// CmdDashboard is cmd_dashboard: the pinned status issue: once, or every `--every` seconds for
// `--for` seconds (the `bot-status` loop). With `--sweep` each tick sweeps first, so the bot does
// not wait hours for GitHub's late schedules to start its next run when its chain of runs breaks.
// Each tick reads the subscriptions and the secrets afresh (`_current`), so one added or changed
// while the loop runs shows within a tick (or, when this checkout's code is too old to read it,
// the loop ends and the next one shows it on newer code), and settles the issue about the
// machine's disk (`disk.alert`). With `--stats` the issue carries the statistics, counted afresh
// every 30 minutes and kept between (`dashboard.update`). A failure is only a warning: it never
// fails the sweep or the loop.
//
// With `--companions`, each other bot (Squishy, #60) is swept (with `--sweep`) and drawn by a
// process of its own (`_companions`), and its section goes into the issue apart from the night
// bot's. `--section-only` is that process: it prints this bot's section and writes nothing. With
// `--runners-every`, the issue's runners section is redrawn alone that often between the rewrites
// (`dashboard.refresh_runners`), so it is never older than that.
func CmdDashboard(cfg *config.Config, args *Args) int {
	if args.SectionOnly {
		say(dashboard.Section(Ctx(cfg, false)))
		return 0
	}
	every := max(0, args.Every)
	runners := max(0, args.RunnersEvery)
	deadline := Monotonic() + float64(max(0, args.ForSeconds))
	since := runCreated(cfg)
	if args.Controller && cfg.Controller {
		loop := *cfg
		loop.ControllerLoop = true // this loop plans (#1455)
		cfg = &loop
	}
	for {
		fresh := current(cfg, since)
		if fresh == nil {
			return 0 // bot-status starts the next loop, on the default branch's code
		}
		cfg = fresh
		if args.Sweep {
			if err := herr.Try(func() {
				notes := Sweep(Ctx(cfg, true))
				if len(notes) == 0 {
					notes = []string{"nothing was left unanswered"}
				}
				for _, line := range notes {
					say(redact.Redact("sweep: "+line, nil))
				}
			}); err != nil {
				// one bad tick must not end the loop
				say(redact.Redact("::warning::the sweep failed: "+herr.Str(err), nil))
			}
		}
		var extra []string
		if args.Companions {
			extra = companions(cfg, args.Sweep, []string{"dashboard", "--section-only"}, "section")
		}
		if err := herr.Try(func() {
			note := dashboard.Update(Ctx(cfg, true), extra, args.Stats, statsSections(cfg, args), false,
				cardsets.FetchJSON)
			say(redact.Redact("dashboard: "+note, nil))
		}); err != nil {
			say(redact.Redact("::warning::the status issue was not updated: "+herr.Str(err), nil))
		}
		if err := herr.Try(func() {
			if note := disk.Alert(Ctx(cfg, true)); note != "" {
				say(redact.Redact("disk: "+note, nil))
			}
		}); err != nil {
			say(redact.Redact("::warning::the disk issue was not settled: "+herr.Str(err), nil))
		}
		if every == 0 || Monotonic()+float64(every) > deadline {
			return 0
		}
		due := Monotonic() + float64(every)
		for runners != 0 && Monotonic()+float64(runners) < due {
			Sleep(float64(runners))
			if err := herr.Try(func() {
				say(redact.Redact("runners: "+dashboard.RefreshRunners(Ctx(cfg, true)), nil))
			}); err != nil {
				say(redact.Redact("::warning::the runners were not redrawn: "+herr.Str(err), nil))
			}
		}
		Sleep(max(0.0, due-Monotonic()))
	}
}

// statsSections is _stats_sections: what draws the other bots' records in the statistics: their
// own processes with `--companions` (#60), nothing without. Called only when the statistics are
// counted.
func statsSections(cfg *config.Config, args *Args) func() []string {
	if !args.Companions {
		return func() []string { return nil } // `tuple`: called, an empty tuple
	}
	return func() []string {
		return companions(cfg, false, []string{"stats", "--section-only"}, "statistics")
	}
}

// CmdStats is cmd_stats: the statistics in the pinned status issue (stats.py): the issue
// rewritten now, with them counted afresh when due, or at once with --force. `--section-only`
// prints this bot's record (Squishy's, which the night bot's loop puts in) and writes nothing;
// `--companions` draws the other bots' sections and records, as the loop does.
func CmdStats(cfg *config.Config, args *Args) int {
	if args.SectionOnly {
		say(stats.BotSection(Ctx(cfg, false)))
		return 0
	}
	var extra []string
	if args.Companions {
		extra = companions(cfg, false, []string{"dashboard", "--section-only"}, "section")
	}
	say(redact.Redact("dashboard: "+dashboard.Update(Ctx(cfg, true), extra, true, statsSections(cfg, args),
		args.Force, cardsets.FetchJSON), nil))
	return 0
}

// CmdHalt is cmd_halt.
func CmdHalt(cfg *config.Config, args *Args) int {
	ctx := Ctx(cfg, true)
	reason := strings.Join(args.Reason, " ")
	if reason == "" {
		reason = "halted from the command line"
	}
	ctx.Store.Update(func(s *pyjson.Object) any {
		s.Set("halted", true)
		s.Set("halt", pyjson.New("by", "cli", "at", clock.ISO(ctx.Now()), "reason", reason))
		return nil
	}, "halt")
	say("halted")
	return 0
}

// CmdStart is cmd_start.
func CmdStart(cfg *config.Config, args *Args) int {
	ctx := Ctx(cfg, true)
	ctx.Store.Update(func(s *pyjson.Object) any {
		s.Set("halted", false)
		s.Set("halt", pyjson.New())
		return nil
	}, "start")
	extra := ""
	if ctx.RepoHalted() {
		extra = " (but .harness/HALT is on main)"
	}
	say("started" + extra)
	return 0
}

// CmdDispatch is cmd_dispatch.
func CmdDispatch(cfg *config.Config, args *Args) int {
	ctx := Ctx(cfg, true)
	var item *int
	shown := "None"
	if args.Item != "" {
		n := py.MustInt(args.Item)
		item, shown = &n, strconv.Itoa(n)
	}
	ctx.Dispatch(item, args.Force, args.Mode)
	say(fmt.Sprintf("dispatched %s (item=%s, force=%s, mode=%s)", config.NightWorkflow, shown,
		py.Str(args.Force), args.Mode))
	return 0
}

// CmdProviders is cmd_providers: each subscription: its CLI and model, hours, limits, and whether
// it could start now.
func CmdProviders(cfg *config.Config, args *Args) int {
	now := clock.Now(cfg.NowOverride)
	st := pyjson.New()
	if !args.Offline {
		if exc, ok := herr.CatchHarness(func() { st = Ctx(cfg, false).Store.Load() }); ok {
			say(fmt.Sprintf("(the state file could not be read: %s; usage is not shown)", exc.Error()))
		}
	}
	for _, provider := range cfg.Pool.Ordered() {
		reason := providers.StartReason(provider, st, now, cfg.Timezone, cfg.Secrets)
		limits := provider.Limits
		var caps []string
		limits.Stops().Items(func(k string, v any) bool {
			f, _ := pyjson.AsFloat(v)
			caps = append(caps, fmt.Sprintf("%s %.0f%%", k, f*100))
			return true
		})
		limits.Budgets().Items(func(k string, v any) bool {
			caps = append(caps, fmt.Sprintf("%s %s min", k, py.Str(v)))
			return true
		})
		capText := strings.Join(caps, ", ")
		if capText == "" {
			capText = "none"
		}
		var seats []string
		for _, seat := range cfg.Pool.Seats(provider) {
			s := seat.Model + " " + seat.Tier
			if seat.SelfCheck {
				s += " self-check"
			}
			seats = append(seats, s)
		}
		ready := "ready"
		if reason != "" {
			ready = reason
		}
		say(fmt.Sprintf("%s %s %s hours: %s limits: %s %s", py.Ljust(provider.ID, 10), py.Ljust(provider.CLI, 7),
			py.Ljust(strings.Join(seats, ", "), 36), py.Ljust(provider.Hours(cfg.Timezone), 34), py.Ljust(capText, 28), ready))
	}
	say(fmt.Sprintf("at most %d at once on GitHub's runners and %d on the machine; priority %s",
		cfg.Pool.MaxParallel, cfg.Pool.MachineParallel, strings.Join(cfg.Pool.Priority, ", ")))
	// A tier's entries name models; whether one checks itself is its subscription's seat's.
	checking := map[string]bool{}
	for _, provider := range cfg.Pool.Ordered() {
		for _, seat := range cfg.Pool.Seats(provider) {
			if seat.SelfCheck {
				checking[seat.Model] = true
			}
		}
	}
	for _, tier := range providers.Tiers {
		var order []string
		for _, e := range cfg.Pool.TierEntries(tier) {
			s := e.Model
			if checking[e.Model] {
				s += " (self-check)"
			}
			order = append(order, s)
		}
		text := strings.Join(order, ", ")
		if text == "" {
			text = "none"
		}
		say(fmt.Sprintf("%s tier, tried in this order: %s", tier, text))
	}
	return 0
}

// CmdDoctor is cmd_doctor.
func CmdDoctor(cfg *config.Config, args *Args) int {
	var errs, warnings, ok []string
	ctx := Ctx(cfg, true)
	for _, p := range ctx.Trust.Problems {
		errs = append(errs, "trust: "+p)
	}
	if len(ctx.Trust.Entries) == 0 {
		errs = append(errs, "trust: nobody is on .harness/trust.txt")
	} else {
		ok = append(ok, "trust: "+strings.Join(ctx.Trust.Logins(), ", "))
	}
	for _, p := range work.CheckTemplates() {
		errs = append(errs, "prompts: "+p)
	}
	if cfg.BotToken != "" {
		if exc, failed := herr.Catch[*herr.GitHubError](func() {
			who := py.Str(ctx.GH.Viewer().Get("login"))
			line := fmt.Sprintf("BOT_GITHUB_TOKEN belongs to @%s (expected @%s)", who, cfg.BotLogin)
			if strings.ToLower(who) == strings.ToLower(cfg.BotLogin) {
				ok = append(ok, line)
			} else {
				warnings = append(warnings, line)
			}
		}); failed {
			errs = append(errs, "BOT_GITHUB_TOKEN does not work: "+exc.Error())
		}
	} else if cfg.ActionsToken != "" && args.Work {
		ok = append(ok, "no GitHub write token here, as intended: the model job never holds one")
	} else if cfg.ActionsToken != "" {
		warnings = append(warnings, "no BOT_GITHUB_TOKEN: GitHub writes use the Actions token, so comments "+
			"come from github-actions[bot] and a pull request the bot opens does not "+
			"start CI (so auto-merge never fires)")
	} else {
		warnings = append(warnings, "no GitHub token in the environment")
	}
	if args.Work {
		provider := cfg.Pool.Ordered()[0]
		if args.Plan != "" {
			planned := object(readJSON(args.Plan))
			if got := cfg.Pool.Get(pyjson.Str(planned.Get("provider"))); got != nil {
				provider = got
			}
		}
		binary := cfg.Bin(provider.CLI)
		if _, err := exec.LookPath(binary); err != nil {
			errs = append(errs, fmt.Sprintf("`%s` (%s's CLI) is not on PATH", binary, provider.ID))
		} else {
			version, err := SubprocessRun([]string{binary, "--version"}, RunOptions{})
			herr.Must(err)
			text := version.Stdout
			if text == "" {
				text = version.Stderr
			}
			ok = append(ok, fmt.Sprintf("%s: %s", provider.CLI, py.SliceTo(strings.TrimSpace(text), 80)))
		}
		if provider.Login == "machine" {
			ok = append(ok, provider.ID+" uses the login on this machine")
		} else if cfg.SecretFor(provider.Secret) == "" {
			errs = append(errs, fmt.Sprintf("%s's secret (%s) is not in this job", provider.ID, provider.Secret))
		}
	}
	if exc, failed := herr.Catch[*herr.GitHubError](func() {
		info := ctx.GH.RepoInfo()
		// Only a token with push access sees this field; a read-only token cannot tell.
		if info.Has("allow_auto_merge") && !info.Truthy("allow_auto_merge") {
			warnings = append(warnings, "the repository does not allow auto-merge (setup --repo-settings)")
		}
		required := ctx.GH.RequiredChecks(cfg.DefaultBranch)
		if required == nil {
			warnings = append(warnings, fmt.Sprintf("`%s` has no branch protection, so auto-merge ", cfg.DefaultBranch)+
				"cannot wait for CI (setup --repo-settings)")
		} else {
			missing := []string{}
			for _, c := range cfg.RequiredChecks {
				if !required[c] {
					missing = append(missing, c)
				}
			}
			if len(missing) > 0 {
				warnings = append(warnings, fmt.Sprintf("required checks missing on %s: %s", cfg.DefaultBranch, py.Repr(missing)))
			} else {
				ok = append(ok, "branch protection requires every CI check")
			}
		}
	}); failed {
		status := "None"
		if exc.Status != 0 {
			status = strconv.Itoa(exc.Status)
		}
		warnings = append(warnings, "could not read the repository settings: "+status)
	}
	if exc, failed := herr.Catch[*herr.GitHubError](func() {
		names := map[string]bool{}
		for _, label := range ctx.GH.ListLabels() {
			names[py.Str(label.Get("name"))] = true
		}
		missingLabels := []string{}
		for _, l := range config.LabelSpecs() {
			if !names[l.Name] {
				missingLabels = append(missingLabels, l.Name)
			}
		}
		if len(missingLabels) > 0 {
			warnings = append(warnings, "labels missing: "+py.Repr(missingLabels))
		} else {
			ok = append(ok, "labels: all present")
		}
		stateBranch := config.StateBranch
		if ctx.GH.BranchSHA(stateBranch) == "" {
			warnings = append(warnings, fmt.Sprintf("no `%s` branch yet (setup creates it)", stateBranch))
		}
		st := ctx.Store.Load()
		if st.Truthy("halted") {
			warnings = append(warnings, "halted: "+py.Str(st.Get("halt")))
		}
		for _, provider := range cfg.Pool.Ordered() {
			refusal := providers.Refusal(provider, providers.PeekRecord(st, provider.ID), ctx.Now(), cfg.Timezone,
				false, time.Time{})
			if refusal != "" {
				warnings = append(warnings, fmt.Sprintf("%s: %s", provider.ID, refusal))
			}
		}
		if ctx.RepoHalted() {
			warnings = append(warnings, ".harness/HALT is on main")
		}
	}); failed {
		warnings = append(warnings, "could not read labels or state: "+exc.Error())
	}
	for _, line := range ok {
		say("ok       " + line)
	}
	for _, line := range warnings {
		say("warning  " + line)
	}
	for _, line := range errs {
		say("ERROR    " + line)
	}
	if len(errs) > 0 {
		return 1
	}
	return 0
}

// CmdSetup is cmd_setup.
func CmdSetup(cfg *config.Config, args *Args) int {
	ctx := Ctx(cfg, true)
	// `LABELS` is the one list (#187): a missing label is created, and one whose colour or
	// description someone changed by hand is brought back to it.
	for _, l := range config.LabelSpecs() {
		if done := ctx.GH.SyncLabel(l.Name, l.Color, l.Description); done != "" {
			say(fmt.Sprintf("%s label %s", done, l.Name))
		}
	}
	if ctx.Store.Ensure() {
		say("created branch " + config.StateBranch)
	}
	if args.RepoSettings {
		ctx.GH.UpdateRepo(pyjson.New("allow_auto_merge", true, "delete_branch_on_merge", true))
		say("repository: auto-merge allowed, merged branches deleted")
		ctx.GH.SetProtection(cfg.DefaultBranch, append([]string(nil), cfg.RequiredChecks...))
		say(fmt.Sprintf("%s: requires %d checks", cfg.DefaultBranch, len(cfg.RequiredChecks)))
	}
	say("setup done")
	return 0
}

// CmdTriage is cmd_triage: triage a new issue or pull request, an issue a person labelled
// `human`, or a thread a person called it on (`triage.py`). Any failure skips quietly: triage is a
// convenience, never a reason for a red run.
func CmdTriage(cfg *config.Config, args *Args) int {
	payload := object(readJSON(args.Payload))
	asked := strings.TrimLeft(strings.TrimSpace(py.OrStr(payload.Obj("inputs").Get("number"), "")), "#")
	var number int
	var done []string
	var decided *triage.Plan
	if exc, failed := catchIf(func() {
		ctx := Ctx(cfg, true)
		if asked != "" { // `workflow_dispatch`: a person called triage on this thread
			payload = pyjson.New("issue", ctx.GH.GetIssue(py.MustInt(asked)))
		}
		step, why := triage.Gate(payload, trust.TrustLoad(filepath.Join(cfg.Root, config.TrustPath)),
			cfg.BotLogin, asked != "")
		shown := step
		if shown == "" {
			shown = "skip"
		}
		say(fmt.Sprintf("triage: %s: %s", shown, why))
		if step == "" {
			return
		}
		thread, _ := triage.ThreadOf(payload)
		number = py.MustInt(py.Item(thread, "number"))
		issue := ctx.GH.GetIssue(number) // afresh: what a person set meanwhile counts
		isPR := issue.Has("pull_request")
		var types map[string]string
		if !isPR {
			types = triage.IssueTypes(ctx.GH)
		}
		decided = triage.Decide(step, issue, isPR, cfg.BotLogin, types, asked == "")
		done = triage.Apply(ctx.GH, number, decided)
	}, func(err error) bool {
		_, gh := err.(*herr.GitHubError)
		return gh || isValueError(err)
	}); failed {
		say(redact.Redact("triage: could not triage: "+exc.Error(), nil))
		return 0
	}
	if decided == nil {
		return 0 // the gate skipped it
	}
	lines := done
	if len(lines) == 0 {
		lines = []string{"nothing to change"}
	}
	report := fmt.Sprintf("Triage of #%d:\n", number) + bullets(append(append([]string(nil), lines...), decided.Notes...))
	say(report)
	Summary(report)
	return 0
}

// CmdForget is cmd_forget: clear an item's failure count and findings, so it can be queued fresh.
func CmdForget(cfg *config.Config, args *Args) int {
	ctx := Ctx(cfg, true)
	number := py.MustInt(args.Number)
	ctx.Store.Update(func(s *pyjson.Object) any {
		rec := state.Item(s, number)
		rec.Set("failures", 0)
		rec.Set("last_findings", []any{})
		return nil
	}, fmt.Sprintf("forget #%d", number))
	say(fmt.Sprintf("#%d: failures and findings cleared", number))
	return 0
}

// Parser is parser.
func Parser() *ArgumentParser {
	top := newParser("harness", doc)
	sub := top.subparsers("command", true)
	p := sub.addParser(top, "plan", "decide what this night run does, and claim it")
	p.option("--out", nil, "").required = true
	p.flag("--force", "ignore the subscriptions' hours")
	p.option("--quiet-ok", nil, "the gate's verdict: the shared subscription it saw quiet, or empty")
	p.option("--item", "", "")
	p.option("--mode", "auto", "").choices = plan.Modes
	p = sub.addParser(top, "peek", "would a run find work? changes nothing")
	p.flag("--force", "ignore the subscriptions' hours")
	p.option("--item", "", "")
	p.option("--mode", "auto", "").choices = plan.Modes
	sub.addParser(top, "quiet", "wait until nobody else is spending the subscription")
	p = sub.addParser(top, "wake", "start the Claude sessions that are not running (wake.py)")
	p.flag("--wait", "sleep until the wake time, or say there is nothing to do (the default)")
	p.flag("--now", "wake at once, at any hour")
	p.flag("--list", "print this bot's subscriptions to wake")
	p.option("--ping", "", "wake this one subscription with the job's secret")
	p = sub.addParser(top, "work", "the model job: build, review and fix one item")
	p.option("--plan", nil, "").required = true
	p.option("--out", nil, "").required = true
	p.option("--work-dir", nil, "").required = true
	p = sub.addParser(top, "plan-fetch", "read the plan the status loop made for this run (#1455)")
	p.option("--ref", nil, "the run's `plan_ref` input").required = true
	p.option("--out", nil, "").required = true
	p.flag("--check", "hold the job's runner, CLI, secret and shared flag to the plan's")
	p = sub.addParser(top, "deliver", "check the model job's output and publish it")
	p.option("--plan", nil, "").required = true
	p.option("--out", nil, "").required = true
	p.option("--action", "", "the plan job's action output, which wins")
	p.option("--number", "", "the plan job's number output, which wins")
	p.option("--provider", "", "the plan job's provider output, which wins")
	p = sub.addParser(top, "event", "handle one GitHub event")
	p.option("--name", "", "")
	p.option("--payload", "", "")
	sub.addParser(top, "sweep", "answer any request an event handler never answered")
	sub.addParser(top, "status", "print the status report")
	dash := sub.addParser(top, "dashboard", "rewrite the pinned status issue (bot-status "+
		"runs it every five minutes, its runners every two)")
	dash.option("--every", 0, "seconds between rewrites; 0 rewrites it once").isInt = true
	dash.option("--runners-every", 0, "seconds between redraws of the runners section alone, between "+
		"the rewrites; 0 draws it only with them").isInt = true
	forSeconds := dash.option("--for", 0, "how long to keep rewriting it, in seconds")
	forSeconds.isInt, forSeconds.dest = true, "for_seconds"
	dash.flag("--sweep", "sweep before each rewrite, as the ten-minute sweep does")
	dash.flag("--stats", "carry the statistics in it, counted afresh every 30 minutes")
	dash.flag("--companions", "also sweep and draw each other bot (Squishy) in a process of "+
		"its own, and put its section in the issue")
	dash.flag("--controller", "with the config's `controller` on, plan the work and start each "+
		"night run with its plan in the sweep (#1455)")
	dash.flag("--section-only", "print this bot's section of the status issue and write nothing "+
		"(the night bot's loop runs this as Squishy)")
	p = sub.addParser(top, "stats", "rewrite the pinned status issue now, with its statistics")
	p.flag("--force", "count the statistics afresh even if they were counted under 30 minutes ago")
	p.flag("--companions", "also put in each other bot's section and record (Squishy's), drawn by "+
		"its own process")
	p.flag("--section-only", "print this bot's record for the statistics and write nothing")
	p = sub.addParser(top, "halt", "stop all model work")
	p.positional("reason", nargsAny)
	sub.addParser(top, "start", "lift a halt")
	p = sub.addParser(top, "dispatch", "start a night run now")
	p.option("--item", "", "")
	p.flag("--force", "")
	p.option("--mode", "auto", "").choices = plan.Modes
	for _, name := range []string{"providers", "window"} {
		p = sub.addParser(top, name, "each subscription's hours, limits and readiness")
		p.flag("--offline", "do not read the state file")
	}
	p = sub.addParser(top, "doctor", "check the configuration and the repository")
	p.flag("--work", "also check the model job's CLI and secret")
	p.option("--plan", "", "the plan file, for the provider to check")
	p = sub.addParser(top, "setup", "create or update the labels, and create the state branch")
	p.flag("--repo-settings", "also allow auto-merge and protect the default branch (needs admin)")
	p = sub.addParser(top, "triage", "label, type and assign a new issue or pull request, an "+
		"issue labelled human, or one a person called it on")
	p.option("--payload", nil, "the event's payload file").required = true
	p = sub.addParser(top, "forget", "clear an item's failure count")
	p.positional("number", nargsOne)
	return top
}

// doc is the module's docstring, the top parser's description.
const doc = "The command line: `harness <command>` from the `bot/` directory.\n\n" +
	"The workflows call `peek`, `quiet`, `plan`, `work`, `deliver`, `event` and `sweep`. An operator\n" +
	"calls `status`, `providers`, `halt`, `start`, `dispatch`, `doctor`, `setup` and `forget`, with a\n" +
	"token in BOT_GITHUB_TOKEN, GITHUB_TOKEN or GH_TOKEN, or a logged-in `gh`.\n"

// Commands is COMMANDS.
var Commands = map[string]func(cfg *config.Config, args *Args) int{
	"peek":       CmdPeek,
	"quiet":      CmdQuiet,
	"wake":       CmdWake,
	"plan":       CmdPlan,
	"work":       CmdWork,
	"plan-fetch": CmdPlanFetch,
	"deliver":    CmdDeliver,
	"event":      CmdEvent,
	"sweep":      CmdSweep,
	"status":     CmdStatus,
	"dashboard":  CmdDashboard,
	"stats":      CmdStats,
	"halt":       CmdHalt,
	"start":      CmdStart,
	"dispatch":   CmdDispatch,
	"providers":  CmdProviders,
	"window":     CmdProviders,
	"doctor":     CmdDoctor,
	"setup":      CmdSetup,
	"forget":     CmdForget,
	"triage":     CmdTriage,
}

// loadConfig is `config_mod.load()`; a variable so this package's own tests can run `main`
// without a checkout's config.
var loadConfig = func() *config.Config { return config.Load("", nil) }

// Main is main, with `sys.exit`'s code as its result: argparse's 0 or 2, the command's, 1 for
// what `main` catches (a ConfigError or HarnessError) and 1, with a traceback's last line on
// stderr, for anything else, as an uncaught exception exits Python. A nil `argv` is the process's
// own arguments.
func Main(argv []string /* = None */) (code int) {
	defer func() {
		if r := recover(); r != nil {
			stack := debug.Stack()
			fmt.Fprintf(Stderr, "Traceback (most recent call last):\n%s%s\n", stack, lastLine(r))
			code = 1
		}
	}()
	if argv == nil {
		argv = os.Args[1:]
	}
	if len(argv) > 0 {
		switch argv[0] {
		case "titles": // harness titles …
			return titles.Main(argv[1:])
		case "pr-title": // pr-title.yml's check
			return prTitle(argv[1:])
		}
	}
	args, exit, ok := Parser().ParseArgs(argv)
	if !ok {
		return exit
	}
	result := 0
	if exc, caught := herr.CatchHarness(func() {
		cfg := loadConfig()
		result = Commands[args.Command](cfg, args)
	}); caught {
		sayErr(redact.Redact("error: "+exc.Error(), nil))
		return 1
	}
	return result
}

// prTitle is pr-title.yml's check (#187): a pull request's title becomes its squash commit's
// subject, so it must follow docs/issues-and-patches.md (Titles). The title, the server and the
// repository come as `--title`, `--server-url` and `--repository` (the workflow's TITLE,
// GITHUB_SERVER_URL and GITHUB_REPOSITORY); the step summary is the job's.
func prTitle(argv []string) int {
	p := newParser("harness pr-title", "")
	p.option("--title", nil, "the pull request's title").required = true
	p.option("--server-url", "", "GITHUB_SERVER_URL, for the summary's link")
	p.option("--repository", "", "GITHUB_REPOSITORY, for the summary's link")
	ns, exit := p.parse(argv)
	if exit != nil {
		io.WriteString(Stdout, exit.stdout)
		io.WriteString(Stderr, exit.stderr)
		return exit.code
	}
	title, server, repository := ns["title"].(string), ns["server_url"].(string), ns["repository"].(string)
	if triage.PullTitleOK(title) {
		say("The title follows the convention: " + title)
		return 0
	}
	say("::error title=Pull request title::" + py.Repr(title) + " breaks docs/issues-and-patches.md (Titles). " +
		"Retitle it, usually as its issue is: Patch: <what it does>, or Patch (part n of m): <...> " +
		"for a part. No title names a version before it ships.")
	path := GithubEnv()["GITHUB_STEP_SUMMARY"]
	if path == "" {
		panic(py.KeyError("GITHUB_STEP_SUMMARY"))
	}
	appendText(path, "### The title breaks the convention\n\n`"+title+"`\n\n"+
		"Titles follow [docs/issues-and-patches.md]("+server+"/"+
		repository+"/blob/HEAD/docs/issues-and-patches.md#titles): "+
		"`Patch: <what it does>`, or `Patch (part n of m): …` for a part; `patches ship` "+
		"names the version of a card change when it ships. Edit the title and this check runs again.\n")
	return 1
}
