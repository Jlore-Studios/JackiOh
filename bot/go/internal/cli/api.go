// Package cli is bot/harness/__main__.py.
//
// The command line: `python -m harness <command>` from the `bot/` directory.
//
// The workflows call `peek`, `quiet`, `plan`, `work`, `deliver`, `event` and `sweep`. An operator
// calls `status`, `providers`, `halt`, `start`, `dispatch`, `doctor`, `setup` and `forget`, with a
// token in BOT_GITHUB_TOKEN, GITHUB_TOKEN or GH_TOKEN, or a logged-in `gh`.
package cli

import (
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"time"
)

// _dump is __main__._dump.

func Dump(obj any) {
	panic("unported: __main__._dump")
}

// LOCK_TRIES = 30
var LockTries int = 30

// LOCK_WAIT = 10.0
var LockWait float64 = 10.0

// cmd_plan is __main__.cmd_plan.

func CmdPlan(cfg *config.Config, args any) int {
	panic("unported: __main__.cmd_plan")
}

// cmd_peek is __main__.cmd_peek.

func CmdPeek(cfg *config.Config, args any) int {
	panic("unported: __main__.cmd_peek")
}

// cmd_quiet is __main__.cmd_quiet.

func CmdQuiet(cfg *config.Config, args any) int {
	panic("unported: __main__.cmd_quiet")
}

// The wake (`wake.py`): `--wait` sleeps until the wake time, `--list` prints this
// bot's subscriptions to wake as JSON on its last line, and `--ping` wakes one with the job's
// own secret.

func CmdWake(cfg *config.Config, args any) int {
	panic("unported: __main__.cmd_wake")
}

// Between a run's steps: whether it must stop. `last_hour` gives the reset of the 5-hour
// session whose last hour the run took (`work.Worker.last_hour`), whose cap stays lifted.

func MakeProbe(ctx *hctx.Context, number *int, provider *providers.Provider /* = None */, lastHour func() time.Time /* = lambda: None */) any {
	panic("unported: __main__.make_probe")
}

// cmd_work is __main__.cmd_work.

func CmdWork(cfg *config.Config, args any) int {
	panic("unported: __main__.cmd_work")
}

// A night run the status loop started with its plan (#1455): read the plan from the state
// branch into `--out`. With `--check` (the model job), the runner, CLI, secret name and shared
// flag the job was given (`PLAN_RUNS_ON`, `PLAN_CLI`, `PLAN_SECRET`, `PLAN_SHARED`) must be the
// plan's and its subscription's in providers.json, or the job stops before any step has a
// secret.

func CmdPlanFetch(cfg *config.Config, args any) int {
	panic("unported: __main__.cmd_plan_fetch")
}

// cmd_deliver is __main__.cmd_deliver.

func CmdDeliver(cfg *config.Config, args any) int {
	panic("unported: __main__.cmd_deliver")
}

// cmd_event is __main__.cmd_event.

func CmdEvent(cfg *config.Config, args any) int {
	panic("unported: __main__.cmd_event")
}

// cmd_sweep is __main__.cmd_sweep.

func CmdSweep(cfg *config.Config, args any) int {
	panic("unported: __main__.cmd_sweep")
}

// cmd_status is __main__.cmd_status.

func CmdStatus(cfg *config.Config, args any) int {
	panic("unported: __main__.cmd_status")
}

// Run `python -m harness <argv>` as another bot (`config.OTHERS`, #60): its home, its token.
// One process is one bot, since the labels and branches are fixed when it starts.

func Companion(other any, argv ...string) any {
	panic("unported: __main__._companion")
}

// COMPANION_TIMEOUT = 300
var CompanionTimeout int = 300

// The pinned status issue: once, or every `--every` seconds for `--for` seconds (the
// `bot-status` loop). With `--sweep` each tick sweeps first, so the bot does not wait hours
// for GitHub's late schedules to start its next run when its chain of runs breaks. Each tick
// reads the subscriptions and the secrets afresh (`_current`), so one added or changed while
// the loop runs shows within a tick (or, when this checkout's code is too old to read it, the
// loop ends and the next one shows it on newer code), and settles the issue about the
// machine's disk (`disk.alert`). With `--stats` the issue carries the statistics, counted
// afresh every 30 minutes and kept between (`dashboard.update`). A failure is only a warning:
// it never fails the sweep or the loop.
//
// With `--companions`, each other bot (Squishy, #60) is swept (with `--sweep`) and drawn by a
// process of its own (`_companions`), and its section goes into the issue apart from the night
// bot's. `--section-only` is that process: it prints this bot's section and writes nothing.
// With `--runners-every`, the issue's runners section is redrawn alone that often between the
// rewrites (`dashboard.refresh_runners`), so it is never older than that.

func CmdDashboard(cfg *config.Config, args any) int {
	panic("unported: __main__.cmd_dashboard")
}

// The statistics in the pinned status issue (stats.py): the issue rewritten now, with them
// counted afresh when due, or at once with --force. `--section-only` prints this bot's record
// (Squishy's, which the night bot's loop puts in) and writes nothing; `--companions` draws the
// other bots' sections and records, as the loop does.

func CmdStats(cfg *config.Config, args any) int {
	panic("unported: __main__.cmd_stats")
}

// cmd_halt is __main__.cmd_halt.

func CmdHalt(cfg *config.Config, args any) int {
	panic("unported: __main__.cmd_halt")
}

// cmd_start is __main__.cmd_start.

func CmdStart(cfg *config.Config, args any) int {
	panic("unported: __main__.cmd_start")
}

// cmd_dispatch is __main__.cmd_dispatch.

func CmdDispatch(cfg *config.Config, args any) int {
	panic("unported: __main__.cmd_dispatch")
}

// Each subscription: its CLI and model, hours, limits, and whether it could start now.

func CmdProviders(cfg *config.Config, args any) int {
	panic("unported: __main__.cmd_providers")
}

// cmd_doctor is __main__.cmd_doctor.

func CmdDoctor(cfg *config.Config, args any) int {
	panic("unported: __main__.cmd_doctor")
}

// cmd_setup is __main__.cmd_setup.

func CmdSetup(cfg *config.Config, args any) int {
	panic("unported: __main__.cmd_setup")
}

// Triage a new issue or pull request, an issue a person labelled `human`, or a thread a
// person called it on (`triage.py`). Any failure skips quietly: triage is a convenience, never a
// reason for a red run.

func CmdTriage(cfg *config.Config, args any) int {
	panic("unported: __main__.cmd_triage")
}

// Clear an item's failure count and findings, so it can be queued fresh.

func CmdForget(cfg *config.Config, args any) int {
	panic("unported: __main__.cmd_forget")
}

// parser is __main__.parser.

func Parser() any {
	panic("unported: __main__.parser")
}

// COMMANDS = { "peek": cmd_peek, "quiet": cmd_quiet, "wake": cmd_wake, "plan": cmd_plan, "work": cmd_work, "plan-fetch": cmd_plan_fetch, "deliver": cmd_deliver, "
var Commands any // unported

// main is __main__.main.

func Main(argv []string /* = None */) int {
	panic("unported: __main__.main")
}
