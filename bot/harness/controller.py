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

The loop also delivers the runs it started (#1537), so they have no `deliver` job either, which
waited 7 to 25 minutes for a hosted runner on 2026-10-20 to run for under a minute. Each tick, before
it plans, `deliver` takes every such run that has ended (`plan.awaiting_delivery`): it reads the
run's plan from the state branch, downloads the model job's `work` upload through the Actions API
and unzips it, clones the default branch afresh into the runner's scratch directory (the loop's
own checkout is shallow and hours old, and the deliverer checks that the bundle descends from the
commit the work started at), and runs the same `Deliverer` deliver ran, as the run. Each try is
recorded in the state file first (`plan.DELIVERIES`), so one that fails, or ends the loop's
process, is tried again on the next tick, at most `plan.DELIVER_TRIES` times. Until then
housekeeping leaves the item alone; after them, or `plan.DELIVER_WITHIN` after the run ended, it
is housekeeping's, as a dead run's item is. A delivery that fails never stops the tick: git runs
through `subprocess` with its timeouts, and every failure is a note.
"""

from __future__ import annotations

import dataclasses
import io
import json
import re
import secrets
import tempfile
import time
import zipfile
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any, Callable

from harness import config as config_mod
from harness import plan as plan_mod
from harness.clock import iso
from harness.config import NIGHT_WORKFLOW, STATE_BRANCH
from harness.context import Context
from harness.deliver import Deliverer
from harness.errors import GitHubError
from harness.git import Git
from harness.redact import redact

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


# ---------------------------------------------------------------------- delivering (#1537)

#: The model job's upload (bot-night.yml's `work` job): the result, the bundle, the sealed login
#: and the transcripts.
WORK_ARTIFACT = "work"
#: How long the clone of the default branch one delivery works in may take, in seconds.
CLONE_TIMEOUT = 600
#: How many of the loop's deliveries the state file remembers.
KEEP_DELIVERIES = 100


def deliver(ctx: Context, *, workspace: Path | None = None) -> list[str]:
    """Deliver every night run the loop started that has ended and still waits for it
    (`plan.awaiting_delivery`). Each line says what happened; nothing here raises."""
    try:
        runs = ctx.gh.list_runs(NIGHT_WORKFLOW, limit=50)
    except GitHubError as exc:
        return [f"could not list the night runs to deliver: {exc}"]
    scratch = workspace or Path(config_mod.github_env().get("RUNNER_TEMP") or tempfile.gettempdir())
    notes = []
    for run in runs:
        try:
            waiting = plan_mod.awaiting_delivery(ctx, run)
        except GitHubError as exc:
            notes.append(f"could not tell whether run {run.get('id')} waits for delivery: {exc}")
            continue
        if waiting:
            notes.append(_deliver_run(ctx, run, scratch))
    return notes


def _deliver_run(ctx: Context, run: dict[str, Any], scratch: Path) -> str:
    """Deliver one run as its own `deliver` job did: its plan, its model job's output and a fresh
    clone of the default branch, with the run's id on the config, so every link and record names
    the night run. A failure is a note, and the run is tried again on the next tick."""
    run_id = str(run.get("id"))
    what = f"run {run_id}"
    try:
        tries = _count_try(ctx, run_id, plan_mod.loop_ref(run))
    except GitHubError as exc:
        return f"could not deliver {what}: the state file refused its try ({exc})"
    try:
        planned = fetch(ctx, plan_mod.loop_ref(run))
        what = f"{_what(planned)} (run {run_id})"
        scratch.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(prefix=f"deliver-{run_id}-", dir=scratch) as tmp:
            out = Path(tmp) / "out"
            found = _download_work(ctx, run_id, out)
            repo = _clone(ctx, Path(tmp) / "repo")
            as_run = dataclasses.replace(ctx, cfg=dataclasses.replace(ctx.cfg, run_id=run_id))
            deliverer = Deliverer(as_run, planned, out, repo)
            deliverer.chain = False  # this tick's sweep plans the next runs itself
            outcome = deliverer.run()
    except Exception as exc:  # noqa: BLE001 - one delivery failing never stops the tick
        left = plan_mod.DELIVER_TRIES - tries
        then = (f"it is tried again next tick ({left} more)" if left > 0
                else "its item is left to housekeeping, as a dead run's")
        return (f"could not deliver {what}, try {tries} of {plan_mod.DELIVER_TRIES} "
                f"({type(exc).__name__}: {redact(str(exc))[:300]}): {then}")
    status = str(outcome.get("status") or "")
    try:
        _record(ctx, run_id, done=True, status=status)
    except GitHubError as exc:
        return f"delivered {what}: {status}, but its record was refused ({exc})"
    return f"delivered {what}: {status}" + ("" if found else " (its model job left no upload)")


def _count_try(ctx: Context, run_id: str, ref: str) -> int:
    """Record one more try at delivering `run_id` before it starts, so a try that ends the loop's
    process counts too. Returns which try this is."""
    tries = 0

    def change(state: dict[str, Any]) -> None:
        nonlocal tries
        book = state.get(plan_mod.DELIVERIES)
        if not isinstance(book, dict):
            book = state[plan_mod.DELIVERIES] = {}
        entry = book.get(run_id) if isinstance(book.get(run_id), dict) else {}
        tries = int(entry.get("tries") or 0) + 1
        book[run_id] = {**entry, "ref": ref, "tries": tries, "at": iso(ctx.now())}
        for old in sorted(book, key=lambda key: str((book[key] or {}).get("at") or ""))[:-KEEP_DELIVERIES]:
            del book[old]

    ctx.store.update(change, f"delivering run {run_id}")
    return tries


def _record(ctx: Context, run_id: str, **fields: Any) -> None:
    def change(state: dict[str, Any]) -> None:
        book = state.get(plan_mod.DELIVERIES)
        if isinstance(book, dict) and isinstance(book.get(run_id), dict):
            book[run_id].update(fields)

    ctx.store.update(change, f"delivered run {run_id}")


def _download_work(ctx: Context, run_id: str, out: Path) -> bool:
    """Unzip the run's `work` upload into `out`. False when its model job left none (it failed
    before the model started, or was cancelled): the deliverer says so then, as deliver did after
    its download, which was allowed to fail."""
    out.mkdir(parents=True, exist_ok=True)
    found = [a for a in ctx.gh.list_artifacts(run_id)
             if a.get("name") == WORK_ARTIFACT and not a.get("expired")]
    if not found:
        return False
    data = ctx.gh.download_artifact(found[0]["id"])
    root = out.resolve()
    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        for member in archive.infolist():
            if not (root / member.filename).resolve().is_relative_to(root):
                raise ValueError(f"the upload holds a path outside it: {member.filename!r}")
        archive.extractall(root)
    return True


def _clone(ctx: Context, into: Path) -> Path:
    """A fresh clone of the default branch, history and all but blobs fetched as needed, for one
    delivery: the deliverer fetches the bundle into it and checks what it descends from."""
    url = f"{ctx.cfg.server_url}/{ctx.cfg.repo}.git"
    Git(into.parent).run("clone", "--quiet", "--filter=blob:none", "--no-tags", "--branch",
                         ctx.cfg.default_branch, url, str(into), timeout=CLONE_TIMEOUT)
    return into
