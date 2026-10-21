"""The status loop's planner (#1455): with `controller` on, `bot-status`'s loop plans the night
bot's work and starts each run with its plan made, so a run no longer waits in GitHub's queue for
a runner for its gate and its plan before its model job (6 to 25 minutes each on 2026-10-20, for
jobs that ran under a minute).

The loop already asked the gate's question every tick (`sweep._night_run`, `plan.peek`); now, when
there is work, it plans itself, under the planning lock (`plan.take_planning_lock`) that the plan
job takes too, and for each run:

- claims the item as a plan job does (`plan.make`), with `pending:<ref>` for the run id, since the
  run it starts has no id yet; `plan.run_status` reads such a claim by the run whose title names
  the reference, and gives it `plan.PENDING_GRACE` to show up;
- writes the plan to the state branch, `plans/<ref>.json`, where only the bots' tokens write, so the
  model job reads it and cannot change it (bot-night.yml's `plan_ref`);
- starts bot-night with `plan_ref` and the plan's runner, CLI, secret name and shared flag, which
  bot-night's gate and plan jobs skip on, and whose model job checks every one of them against
  `providers.json` before any step is handed a secret (`__main__.cmd_plan_fetch`);
- finds the run by its title (`bot-night <ref> on <runner>`) and puts its id on the claim.

It plans again until nothing more can start, as each plan job's `fill_lanes` used to start one
more run, and prunes plan files older than `KEEP_PLANS`. bot-night's own gate and plan still run
for a run started any other way (its schedule, a command, a person), and plan only while no loop
is going (`plan.loop_alive`): rule 13 keeps the schedule.
"""

from __future__ import annotations

import dataclasses
import json
import re
import secrets
import time
from datetime import datetime, timedelta, timezone
from typing import Any, Callable

from harness import plan as plan_mod
from harness.config import NIGHT_WORKFLOW, STATE_BRANCH
from harness.context import Context
from harness.errors import GitHubError

#: Where the loop keeps the plans of the runs it starts, on the state branch.
PLANS = "plans"
#: A plan's reference: random hex, then the unix time of its claim (`plan.pending_run` reads it).
REF = re.compile(r"^[0-9a-f]{8}-[0-9]{9,11}$")
#: How long a plan file stays on the state branch: a run's 350 minutes and a week to read it back.
KEEP_PLANS = timedelta(days=7)
#: How many times, and how far apart in seconds, the loop looks for the run it just started.
FIND_TRIES = 4
FIND_WAIT = 3.0
#: The night runs' titles name the plan they carry (bot-night.yml's `run-name`).
TITLE = "bot-night {ref} on {runs_on}"


def title_prefix(ref: str) -> str:
    return f"bot-night {ref} "


def new_ref(now: datetime) -> str:
    return f"{secrets.token_hex(4)}-{int(now.timestamp())}"


def start(ctx: Context, *, sleep: Callable[[float], None] | None = None) -> list[str]:
    """Plan and start night runs until nothing more can start. Each line says what happened."""
    sleep = sleep or time.sleep
    pool = ctx.cfg.pool
    owner = f"loop {ctx.cfg.run_id or 'by hand'}"
    notes = resolve(ctx)
    if not plan_mod.take_planning_lock(ctx, owner):
        return [*notes, "another planner holds the planning lock: the next tick plans"]
    try:
        for turn in range(pool.max_parallel + pool.machine_parallel + pool.plan_lanes + 1):
            # Each start takes up to half a minute: the lock is taken again, which renews it, so
            # it never ages past `plan.LOCK_TTL` while the loop is still planning.
            if turn and not plan_mod.take_planning_lock(ctx, owner):
                notes.append("another planner took the planning lock: the next tick plans")
                break
            note, started = _start_one(ctx, sleep)
            notes.append(note)
            if not started:
                break
    finally:
        plan_mod.drop_planning_lock(ctx, owner)
    notes.extend(prune(ctx))
    return notes


def _start_one(ctx: Context, sleep: Callable[[float], None]) -> tuple[str, bool]:
    ref = new_ref(ctx.now())
    pending = plan_mod.PENDING + ref
    claiming = dataclasses.replace(ctx, cfg=dataclasses.replace(ctx.cfg, run_id=pending))
    planned = plan_mod.make(claiming, quiet_ok="")
    if planned.get("action") in (None, "", "none"):
        return f"nothing more to start: {planned.get('reason', '')}".rstrip(": "), False
    what = _what(planned)
    runs_on = str(planned.get("runs_on") or "ubuntu-latest")
    inputs = {"plan_ref": ref, "runs_on": runs_on, "cli": str(planned.get("cli") or ""),
              "secret": str(planned.get("secret") or ""),
              "shared": str(bool(planned.get("shared"))).lower(),
              "dry_run": "true" if ctx.cfg.dry_run else "false"}
    try:
        ctx.gh.put_file(f"{PLANS}/{ref}.json", json.dumps(planned, indent=2) + "\n",
                        branch=STATE_BRANCH, sha=None, message=f"plan {ref} [skip ci]")
        ctx.act.dispatch_workflow(NIGHT_WORKFLOW, ctx.cfg.default_branch, inputs)
    except GitHubError as exc:
        _set_run(ctx, planned, pending, "")  # dead: the next housekeeping requeues it
        return f"could not start {what}: {exc}", False
    run_id = find(ctx, ref, sleep)
    if run_id:
        _set_run(ctx, planned, pending, run_id)
        return f"started {what} (run {run_id})", True
    return f"started {what} (its run is not listed yet)", True


def _what(planned: dict[str, Any]) -> str:
    number = planned.get("number")
    action = planned.get("action")
    item = f"{action} #{number}" if number else str(action)
    return f"{item} on {planned.get('provider')}"


def find(ctx: Context, ref: str, sleep: Callable[[float], None]) -> str:
    """The id of the night run whose title names `ref`, or "" when GitHub does not list it yet."""
    for attempt in range(FIND_TRIES):
        try:
            for run in ctx.gh.list_runs(NIGHT_WORKFLOW, limit=20):
                if str(run.get("display_title") or "").startswith(title_prefix(ref)):
                    return str(run.get("id") or "")
        except GitHubError:
            pass
        if attempt < FIND_TRIES - 1:
            sleep(FIND_WAIT)
    return ""


def _set_run(ctx: Context, planned: dict[str, Any], pending: str, run_id: str) -> None:
    """Put `run_id` on the claim `planned` made, if the claim still carries `pending`."""
    def change(state: dict[str, Any]) -> None:
        if planned.get("action") == "suggest":
            entry = state.get("suggest") if isinstance(state.get("suggest"), dict) else {}
        else:
            entry = (state.get("items") or {}).get(str(planned.get("number")), {})
        if isinstance(entry, dict) and entry.get("run_id") == pending:
            entry["run_id"] = run_id

    ctx.store.update(change, f"run of {_what(planned)}")


def resolve(ctx: Context) -> list[str]:
    """Put its run's id on every claim still pending, once GitHub lists the run."""
    state = ctx.store.load()
    waiting = {str(entry.get("run_id"))[len(plan_mod.PENDING):]: number
               for number, entry in (state.get("items") or {}).items()
               if isinstance(entry, dict) and str(entry.get("run_id") or "").startswith(plan_mod.PENDING)}
    survey = state.get("suggest") if isinstance(state.get("suggest"), dict) else {}
    if str(survey.get("run_id") or "").startswith(plan_mod.PENDING):
        waiting[str(survey["run_id"])[len(plan_mod.PENDING):]] = "suggest"
    if not waiting:
        return []
    try:
        runs = ctx.gh.list_runs(NIGHT_WORKFLOW, limit=50)
    except GitHubError:
        return []
    found = {}
    for run in runs:
        title = str(run.get("display_title") or "")
        for ref in waiting:
            if title.startswith(title_prefix(ref)) and run.get("id"):
                found[ref] = str(run["id"])
    if not found:
        return []

    def change(state: dict[str, Any]) -> None:
        for ref, run_id in found.items():
            key = waiting[ref]
            entry = (state.get("suggest") if key == "suggest"
                     else (state.get("items") or {}).get(key))
            if isinstance(entry, dict) and entry.get("run_id") == plan_mod.PENDING + ref:
                entry["run_id"] = run_id

    ctx.store.update(change, "the loop's runs found")
    return [f"found the run of {'the survey' if waiting[ref] == 'suggest' else '#' + str(waiting[ref])}: "
            f"{run_id}" for ref, run_id in found.items()]


def prune(ctx: Context) -> list[str]:
    """Delete the plan files older than `KEEP_PLANS` (their reference holds their time)."""
    try:
        names = ctx.gh.list_dir(PLANS, STATE_BRANCH)
    except (GitHubError, AttributeError):
        return []
    cutoff = ctx.now() - KEEP_PLANS
    gone = 0
    for name, sha in names:
        ref = name[:-len(".json")] if name.endswith(".json") else ""
        if not REF.match(ref):
            continue
        made = datetime.fromtimestamp(int(ref.rsplit("-", 1)[1]), tz=timezone.utc)
        if made >= cutoff:
            continue
        try:
            ctx.gh.delete_file(f"{PLANS}/{name}", branch=STATE_BRANCH, sha=sha,
                               message=f"plan {ref} pruned [skip ci]")
            gone += 1
        except GitHubError:
            continue
    return [f"pruned {gone} plan file(s) older than {KEEP_PLANS.days} days"] if gone else []


def fetch(ctx: Context, ref: str) -> dict[str, Any]:
    """The plan the loop wrote for `ref`, from the state branch. Raises ValueError when the
    reference is malformed or the file is missing or unreadable."""
    if not REF.match(ref):
        raise ValueError(f"{ref!r} is not a plan reference")
    text, _ = ctx.gh.get_file(f"{PLANS}/{ref}.json", STATE_BRANCH)
    if text is None:
        raise ValueError(f"no plan {ref} on {STATE_BRANCH}")
    planned = json.loads(text)
    if not isinstance(planned, dict):
        raise ValueError(f"plan {ref} is not an object")
    return planned


def check(ctx: Context, planned: dict[str, Any], given: dict[str, str]) -> list[str]:
    """What is wrong with a dispatch's `given` runner, CLI, secret name and shared flag against the
    plan and `providers.json`: empty when all agree. The model job runs only on an empty list, so a
    dispatch cannot hand a model job a secret, a CLI or a runner its subscription does not have."""
    problems = []
    provider_id = str(planned.get("provider") or "")
    provider = ctx.cfg.pool.get(provider_id)
    if provider is None:
        return [f"the plan names no subscription providers.json has ({provider_id!r})"]
    shared = provider.quiet_check or provider.id == "claude-1"
    expected = {"runs_on": provider.runs_on or "ubuntu-latest", "cli": provider.cli,
                "secret": provider.secret or "", "shared": str(shared).lower()}
    planned_values = {"runs_on": str(planned.get("runs_on") or "ubuntu-latest"),
                      "cli": str(planned.get("cli") or ""), "secret": str(planned.get("secret") or ""),
                      "shared": str(bool(planned.get("shared"))).lower()}
    for key, want in expected.items():
        if planned_values[key] != want:
            problems.append(f"the plan's {key} is {planned_values[key]!r}, {provider_id}'s is {want!r}")
        if given.get(key, "") != want:
            problems.append(f"the dispatch's {key} is {given.get(key, '')!r}, {provider_id}'s is {want!r}")
    return problems
