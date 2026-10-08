"""The command line: `python -m harness <command>` from the `bot/` directory.

The workflows call `peek`, `quiet`, `plan`, `work`, `deliver`, `event` and `sweep`. An operator
calls `status`, `providers`, `halt`, `start`, `dispatch`, `doctor`, `setup` and `forget`, with a
token in BOT_GITHUB_TOKEN, GITHUB_TOKEN or GH_TOKEN, or a logged-in `gh`.
"""

from __future__ import annotations

import argparse
import dataclasses
import json
import shutil
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

from harness import config as config_mod
from harness import context as context_mod
from harness import dashboard as dashboard_mod
from harness import disk as disk_mod
from harness import stats as stats_mod
from harness import deliver as deliver_mod
from harness import events as events_mod
from harness import logins as logins_mod
from harness import plan as plan_mod
from harness import providers as providers_mod
from harness import quiet as quiet_mod
from harness import status as status_mod
from harness import sweep as sweep_mod
from harness import triage as triage_mod
from harness.clock import iso, now as clock_now, parse_iso
from harness.config import LABELS, TRUST_PATH, Config
from harness.errors import ConfigError, GitHubError, HarnessError, LoginError
from harness.redact import redact
from harness.runner import get_runner, ping_usage
from harness.state import item as state_item
from harness.trust import Trust
from harness.work import Worker, check_templates


def _local_token() -> str:
    """A logged-in `gh`'s token, for an operator running this by hand."""
    gh = shutil.which("gh")
    if not gh:
        return ""
    proc = subprocess.run([gh, "auth", "token"], capture_output=True, text=True, check=False)
    return proc.stdout.strip() if proc.returncode == 0 else ""


def _ctx(cfg: Config, *, write: bool = True) -> context_mod.Context:
    if write and cfg.write_token:
        return context_mod.build(cfg)
    token = cfg.write_token if write else (cfg.actions_token or cfg.bot_token)
    return context_mod.build(cfg, token=token or _local_token())


def _output(pairs: dict[str, Any]) -> None:
    path = config_mod.github_env().get("GITHUB_OUTPUT")
    if not path:
        return
    with open(path, "a", encoding="utf-8") as handle:
        for key, value in pairs.items():
            handle.write(f"{key}={value}\n")


def _summary(text: str) -> None:
    path = config_mod.github_env().get("GITHUB_STEP_SUMMARY")
    if path:
        with open(path, "a", encoding="utf-8") as handle:
            handle.write(redact(text) + "\n")


def _bullets(lines: list[str]) -> str:
    """`lines` as a Markdown list after a blank line, or nothing when there are none."""
    return "\n" + "".join(f"- {line}\n" for line in lines) if lines else ""


def _dump(obj: Any) -> None:
    print(redact(json.dumps(obj, indent=2, default=str)))


# ---------------------------------------------------------------------- workflow commands


def cmd_plan(cfg: Config, args: argparse.Namespace) -> int:
    ctx = _ctx(cfg)
    item = int(args.item) if str(args.item or "").strip().lstrip("#").isdigit() else None
    planned = plan_mod.make(ctx, force=args.force, item=item, mode=args.mode,
                            quiet_ok=args.quiet_ok)
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(planned, indent=2) + "\n", encoding="utf-8")
    # The provider, its CLI and the name of its secret steer the model job: which CLI to install
    # and which one secret to hand it. They come from providers.json, never from the model.
    _output({"action": planned["action"], "number": planned.get("number") or "",
             "provider": planned.get("provider") or "", "cli": planned.get("cli") or "",
             "secret": planned.get("secret") or "",
             "shared": str(bool(planned.get("shared"))).lower(),
             "runs_on": planned.get("runs_on") or "ubuntu-latest"})
    what = planned["action"] if planned["action"] == "none" else (
        f"{planned['action']} #{planned.get('number')}" if planned.get("number") else planned["action"])
    if planned.get("provider"):
        what += f" on {planned['provider']}"
    if planned.get("priority"):
        what += f", priority tier {planned['priority']}"
    if planned.get("difficulty"):
        what += f", difficulty {planned['difficulty']}"
    if planned.get("assignment"):
        what += f" ({planned['assignment']})"
    notes = [*(planned.get("routing") or []), *(planned.get("skipped") or [])]
    _summary(f"### Plan: {what}\n\n{planned.get('reason', '')}\n{_bullets(notes)}")
    print(f"plan: {what} {planned.get('reason', '')}".strip())
    for line in notes:
        print(f"plan: {line}")
    return 0


def cmd_peek(cfg: Config, args: argparse.Namespace) -> int:
    ctx = _ctx(cfg, write=False)
    item = int(args.item) if str(args.item or "").strip().lstrip("#").isdigit() else None
    look = plan_mod.peek(ctx, force=args.force, item=item, mode=args.mode)
    check = bool(look.work and look.quiet_provider)
    _output({"work": str(look.work).lower(), "quiet_check": str(check).lower(),
             "quiet_provider": look.quiet_provider, "quiet_secret": look.quiet_secret,
             "fallback": str(look.fallback).lower()})
    then = (f" First, `{look.quiet_provider}` must be quiet"
            + (" (other work can go ahead if it is not)." if look.fallback else ".")) if check else ""
    _summary(f"### Peek: {'work' if look.work else 'nothing to do'}\n\n{look.reason}.{then}\n"
             f"{_bullets(look.skipped)}")
    print(f"peek: {'work' if look.work else 'nothing'}: {look.reason}.{then}")
    for line in look.skipped:
        print(f"peek: {line}")
    return 0


def cmd_quiet(cfg: Config, args: argparse.Namespace) -> int:
    ctx = _ctx(cfg, write=False)
    token = cfg.actions_token or cfg.bot_token
    verdict = quiet_mod.wait_for_quiet(
        cfg.quiet,
        ping=lambda: ping_usage(cfg.claude_bin, cfg.quiet.ping_model),
        partner=lambda t1, t2: quiet_mod.partner_spending(
            cfg.quiet.partners, lambda repo: quiet_mod.PartnerReader(repo, token), t1, t2),
        now=ctx.now,
        sleep=time.sleep,
    )
    _output({"quiet": str(verdict.quiet).lower()})
    excused = f" (the rise was {verdict.excused_by})" if verdict.excused_by else ""
    _summary(f"### Quiet: {'yes' if verdict.quiet else 'no'}\n\n{verdict.reason}{excused}.\n")
    _dump(verdict.to_dict())
    return 0


def make_probe(ctx: context_mod.Context, number: int | None,
               provider: providers_mod.Provider | None = None):
    provider = provider or ctx.cfg.pool.ordered()[0]

    def closed(thread: int) -> bool:
        try:
            return ctx.gh.get_issue(thread).get("state") == "closed"
        except GitHubError:
            return False  # GitHub could not say: carry on, as before this check

    def probe(last_usage: dict | None):
        if ctx.repo_halted():
            return (f"halted by {config_mod.HALT_PATH} on main", "halt")
        state = ctx.store.load()
        if state.get("halted"):
            return (f"halted by {config_mod.SLASH} halt", "halt")
        held = providers_mod.suspension(state, provider.id)
        if held is not None:
            by = f" by @{held['by']}" if held.get("by") else ""
            return (f"`{provider.id}` was suspended{by}", "suspend")
        if number is not None:
            record = state["items"].get(str(number), {})
            if record.get("stop_requested"):
                return (f"stopped by @{record.get('stopped_by', 'someone')}", "stop")
            if closed(number):
                # Deliver drops a closed thread's work anyway, so stop spending on it now.
                return (f"#{number} was closed", "stop")
        providers_mod.note_usage(state, provider.id, last_usage, None, ctx.now())
        refusal = providers_mod.refusal(provider, providers_mod.peek_record(state, provider.id),
                                        ctx.now(), ctx.cfg.timezone)
        if refusal:
            return (f"`{provider.id}` stops: {refusal}", "usage")
        return None
    return probe


def cmd_work(cfg: Config, args: argparse.Namespace) -> int:
    planned = json.loads(Path(args.plan).read_text(encoding="utf-8"))
    ctx = _ctx(cfg, write=False)
    number = planned.get("number")
    out = Path(args.out)
    provider = cfg.pool.get(planned.get("provider")) or cfg.pool.ordered()[0]
    login = None
    if cfg.backend != "fake":
        secret = cfg.secret_for(provider.secret) if provider.login == "secret" else ""
        if secret or provider.cli != "claude" or provider.login == "machine":
            try:
                login = logins_mod.prepare(provider, secret, str(planned.get("vault") or ""),
                                           Path(args.work_dir) / ".logins" / provider.id)
            except LoginError as exc:
                out.mkdir(parents=True, exist_ok=True)
                failed = {"version": 1, "action": planned.get("action"), "number": number,
                          "status": "infra", "reason": redact(str(exc)), "provider": provider.id}
                (out / "result.json").write_text(json.dumps(failed, indent=2) + "\n",
                                                 encoding="utf-8")
                print(f"work: infra: {redact(str(exc))}")
                return 0
    def keep_login() -> None:
        """After every model call: seal a login the CLI refreshed (and redact its new tokens
        from then on), so even a job killed later hands it on to the next run."""
        if login is None:
            return
        try:
            sealed = logins_mod.seal(login)
        except (ValueError, OSError) as exc:
            print(f"work: could not seal the refreshed login: {redact(str(exc))}")
            return
        if sealed:
            out.mkdir(parents=True, exist_ok=True)
            (out / "vault.enc").write_text(sealed + "\n", encoding="utf-8")

    runner = get_runner(cfg, provider, login)
    worker = Worker(
        cfg, planned, runner, cfg.root, Path(args.work_dir), out,
        probe=make_probe(ctx, int(number) if number else None, provider), after_call=keep_login,
    )
    if cfg.backend != "fake" and provider.limits.stops:
        # A fresh reading before any model work: the stored one is the last run's, and none at
        # all once its window reset. Claude: one Haiku turn, signed in as this subscription; agy
        # and Muse: their own `/usage`, no model call.
        if provider.cli == "claude":
            worker.start_usage = ping_usage(cfg.claude_bin, cfg.quiet.ping_model,
                                            token=secret if provider.login == "secret" else "")
        elif getattr(runner, "polls_usage", False):
            worker.start_usage = runner.read_usage(provider.model)
    result = worker.run()
    keep_login()
    _summary(f"### Work: {result.get('status')}\n\n{result.get('reason', '')}\n")
    print(f"work: {result.get('status')}: {result.get('reason', '')}")
    return 0


def cmd_deliver(cfg: Config, args: argparse.Namespace) -> int:
    planned = json.loads(Path(args.plan).read_text(encoding="utf-8"))
    if (args.action or planned.get("action")) in (None, "", "none"):
        print("deliver: nothing was planned")
        return 0
    ctx = _ctx(cfg)
    number = int(args.number) if str(args.number or "").isdigit() else None
    outcome = deliver_mod.Deliverer(ctx, planned, Path(args.out), cfg.root,
                                    action=args.action or None, number=number,
                                    provider=args.provider or None).run()
    _summary(f"### Deliver: {outcome.get('status')}\n\n" + "\n".join(f"- {l}" for l in outcome["log"]))
    print(f"deliver: {outcome.get('status')}; " + "; ".join(outcome["log"]))
    return 0


def cmd_event(cfg: Config, args: argparse.Namespace) -> int:
    env = config_mod.github_env()
    name = args.name or env.get("GITHUB_EVENT_NAME", "")
    path = args.payload or env.get("GITHUB_EVENT_PATH", "")
    if not name or not path:
        print("event: no event name or payload", file=sys.stderr)
        return 2
    payload = json.loads(Path(path).read_text(encoding="utf-8"))
    for line in events_mod.handle(_ctx(cfg), name, payload):
        print(redact(f"event: {line}"))
    return 0


# ---------------------------------------------------------------------- operator commands


def cmd_sweep(cfg: Config, args: argparse.Namespace) -> int:
    notes = sweep_mod.sweep(_ctx(cfg))
    for line in notes or ["nothing was left unanswered"]:
        print(redact(f"sweep: {line}"))
    _summary("### Sweep\n\n" + "\n".join(f"- {n}" for n in notes or ["nothing was left unanswered"]))
    return 0


def cmd_status(cfg: Config, args: argparse.Namespace) -> int:
    print(status_mod.report(_ctx(cfg)))
    return 0


def _run_created(cfg: Config) -> Any:
    """When this workflow run was created, which is when GitHub fixed its list of secrets; None
    run by hand, or when it cannot be read."""
    if not cfg.run_id:
        return None
    try:
        return parse_iso(_ctx(cfg).gh.get_run(cfg.run_id).get("created_at"))
    except Exception:  # noqa: BLE001 - not knowing only means trusting the newest record
        return None


def _current(cfg: Config, since: Any) -> Config | None:
    """The loop's config as it stands now. The `bot-status` job reads `.harness/` once, when it
    starts, and runs for five and a half hours, and GitHub fixes its secrets when its run is
    created, hours before that for a run queued behind the last loop; so a subscription added or
    changed since would show as it was. Each tick reads the subscriptions again from the default
    branch, and takes which secrets are set from the newest plan job's record when that is newer
    than this run (`providers.newer_secrets`). What cannot be read stays as it was.

    None when this checkout's code refuses the default branch's subscriptions: it read its own
    file when it started, so the branch's has changed since in a way only newer code reads (a
    subscription with a secret this checkout does not know, as claude-7's was), and the loop ends
    so that the next one starts on the default branch's code."""
    pool, secrets = cfg.pool, cfg.secrets
    try:
        ctx = _ctx(cfg)
        text, _ = ctx.gh.get_file(providers_mod.PROVIDERS_PATH.as_posix(), cfg.default_branch)
        if text:
            raw = json.loads(text)
            try:
                pool = providers_mod.parse(raw)
            except ConfigError as exc:
                print(redact(f"::notice::this checkout cannot read the subscriptions on "
                             f"{cfg.default_branch} ({exc}): the next loop starts on its code"),
                      flush=True)
                return None
        secrets = providers_mod.newer_secrets(cfg.secrets, ctx.store.load(), since)
    except Exception as exc:  # noqa: BLE001 - one bad tick must not end the loop
        print(redact(f"::warning::kept the subscriptions and secrets as they were: {exc}"),
              flush=True)
    return dataclasses.replace(cfg, pool=pool, secrets=secrets)


def _companion(other: Any, *argv: str) -> subprocess.CompletedProcess[str]:
    """Run `python -m harness <argv>` as another bot (`config.OTHERS`, #60): its home, its token.
    One process is one bot, since the labels and branches are fixed when it starts."""
    return subprocess.run([sys.executable, "-m", "harness", *argv],
                          cwd=Path(__file__).resolve().parents[1],
                          env=config_mod.companion_env(other), capture_output=True, text=True,
                          timeout=COMPANION_TIMEOUT, check=False)


#: How long a companion's sweep or section may take before the tick goes on without it.
COMPANION_TIMEOUT = 300


def _companions(cfg: Config, *, sweep: bool,
                argv: tuple[str, ...] = ("dashboard", "--section-only"),
                what: str = "section") -> tuple[str, ...]:
    """Each other bot's section (Squishy's, #60), after its sweep when the loop sweeps: its own
    process draws it from its own state and labels, `argv` saying which (its section of the
    status issue, or with `("stats", "--section-only")` of the statistics issue). A bot that
    cannot be run is a warning, and the issue goes on without its section."""
    sections: list[str] = []
    for other in config_mod.OTHERS:
        if not other.home:
            continue
        if sweep and other.token_env and config_mod.companion_env(other).get("BOT_GITHUB_TOKEN"):
            try:
                done = _companion(other, "sweep")
                for line in (done.stdout or "").splitlines():
                    print(redact(f"{other.name}: {line}"), flush=True)
                if done.returncode:
                    print(redact(f"::warning::{other.name}'s sweep failed: "
                                 f"{(done.stderr or '')[-500:]}"), flush=True)
            except (OSError, subprocess.SubprocessError) as exc:
                print(redact(f"::warning::{other.name}'s sweep did not run: {exc}"), flush=True)
        try:
            drawn = _companion(other, *argv)
        except (OSError, subprocess.SubprocessError) as exc:
            print(redact(f"::warning::{other.name}'s {what} was not drawn: {exc}"), flush=True)
            continue
        if drawn.returncode == 0 and drawn.stdout.strip():
            sections.append(drawn.stdout)
        else:
            print(redact(f"::warning::{other.name}'s {what} was not drawn: "
                         f"{(drawn.stderr or '')[-500:]}"), flush=True)
    return tuple(sections)


def cmd_dashboard(cfg: Config, args: argparse.Namespace) -> int:
    """The pinned status issue: once, or every `--every` seconds for `--for` seconds (the
    `bot-status` loop). With `--sweep` each tick sweeps first, so the bot does not wait hours
    for GitHub's late schedules to start its next run when its chain of runs breaks. Each tick
    reads the subscriptions and the secrets afresh (`_current`), so one added or changed while
    the loop runs shows within a tick (or, when this checkout's code is too old to read it, the
    loop ends and the next one shows it on newer code), and settles the issue about the
    machine's disk (`disk.alert`). A failure is only a warning: it never fails the sweep or the loop.

    With `--companions`, each other bot (Squishy, #60) is swept (with `--sweep`) and drawn by a
    process of its own (`_companions`), and its section goes into the issue apart from the night
    bot's. `--section-only` is that process: it prints this bot's section and writes nothing."""
    if getattr(args, "section_only", False):
        print(dashboard_mod.section(_ctx(cfg, write=False)))
        return 0
    every = max(0, int(getattr(args, "every", 0) or 0))
    deadline = time.monotonic() + max(0, int(getattr(args, "for_seconds", 0) or 0))
    since = _run_created(cfg)
    while True:
        fresh = _current(cfg, since)
        if fresh is None:
            return 0  # bot-status starts the next loop, on the default branch's code
        cfg = fresh
        if getattr(args, "sweep", False):
            try:
                for line in sweep_mod.sweep(_ctx(cfg)) or ["nothing was left unanswered"]:
                    print(redact(f"sweep: {line}"), flush=True)
            except Exception as exc:  # noqa: BLE001 - one bad tick must not end the loop
                print(redact(f"::warning::the sweep failed: {exc}"), flush=True)
        extra = (_companions(cfg, sweep=bool(getattr(args, "sweep", False)))
                 if getattr(args, "companions", False) else ())
        try:
            note = dashboard_mod.update(_ctx(cfg), extra)
            print(redact(f"dashboard: {note}"), flush=True)
        except Exception as exc:  # noqa: BLE001 - one bad tick must not end the loop
            print(redact(f"::warning::the status issue was not updated: {exc}"), flush=True)
        try:
            note = disk_mod.alert(_ctx(cfg))
            if note:
                print(redact(f"disk: {note}"), flush=True)
        except Exception as exc:  # noqa: BLE001 - one bad tick must not end the loop
            print(redact(f"::warning::the disk issue was not settled: {exc}"), flush=True)
        if getattr(args, "stats", False):
            try:
                print(redact(stats_mod.update(_ctx(cfg), extra=_stats_sections(cfg, args))),
                      flush=True)
            except Exception as exc:  # noqa: BLE001 - one bad tick must not end the loop
                print(redact(f"::warning::the statistics issue was not updated: {exc}"),
                      flush=True)
        if not every or time.monotonic() + every > deadline:
            return 0
        time.sleep(every)


def _stats_sections(cfg: Config, args: argparse.Namespace) -> Any:
    """What draws the other bots' sections of the statistics issue: their own processes with
    `--companions` (#60), nothing without. Called only when the issue is rewritten."""
    if not getattr(args, "companions", False):
        return tuple
    return lambda: _companions(cfg, sweep=False, argv=("stats", "--section-only"),
                               what="statistics")


def cmd_stats(cfg: Config, args: argparse.Namespace) -> int:
    """The pinned "Night bot statistics" issue (stats.py): rewritten when due, or now with --force.
    `--section-only` prints this bot's section of it (Squishy's, which the night bot's loop puts
    in) and writes nothing; `--companions` puts the other bots' sections in."""
    if getattr(args, "section_only", False):
        print(stats_mod.bot_section(_ctx(cfg, write=False)))
        return 0
    print(redact(stats_mod.update(_ctx(cfg), force=bool(getattr(args, "force", False)),
                                  extra=_stats_sections(cfg, args))))
    return 0


def cmd_halt(cfg: Config, args: argparse.Namespace) -> int:
    ctx = _ctx(cfg)
    reason = " ".join(args.reason) or "halted from the command line"
    ctx.store.update(lambda s: s.update(halted=True, halt={
        "by": "cli", "at": iso(ctx.now()), "reason": reason}), "halt")
    print("halted")
    return 0


def cmd_start(cfg: Config, args: argparse.Namespace) -> int:
    ctx = _ctx(cfg)
    ctx.store.update(lambda s: s.update(halted=False, halt={}), "start")
    print("started" + (" (but .harness/HALT is on main)" if ctx.repo_halted() else ""))
    return 0


def cmd_dispatch(cfg: Config, args: argparse.Namespace) -> int:
    ctx = _ctx(cfg)
    item = int(args.item) if args.item else None
    ctx.dispatch(item=item, force=args.force, mode=args.mode)
    print(f"dispatched {config_mod.NIGHT_WORKFLOW} (item={item}, force={args.force}, mode={args.mode})")
    return 0


def cmd_providers(cfg: Config, args: argparse.Namespace) -> int:
    """Each subscription: its CLI and model, hours, limits, and whether it could start now."""
    now = clock_now(cfg.now_override)
    state: dict[str, Any] = {}
    if not args.offline:
        try:
            state = _ctx(cfg, write=False).store.load()
        except (GitHubError, HarnessError) as exc:
            print(f"(the state file could not be read: {exc}; usage is not shown)")
    for provider in cfg.pool.ordered():
        reason = providers_mod.availability(provider, state, now, cfg.timezone, cfg.secrets)
        limits = provider.limits
        caps = ", ".join([f"{k} {v:.0%}" for k, v in limits.stops.items()]
                         + [f"{k} {v} min" for k, v in limits.budgets.items()]) or "none"
        seats = ", ".join(f"{seat.model} {seat.tier}" + (" self-check" if seat.self_check else "")
                          for seat in cfg.pool.seats(provider))
        print(f"{provider.id:10} {provider.cli:7} {seats:36} "
              f"hours: {provider.hours(cfg.timezone):34} limits: {caps:28} "
              f"{'ready' if reason is None else reason}")
    print(f"at most {cfg.pool.max_parallel} at once on GitHub's runners and "
          f"{cfg.pool.machine_parallel} on the machine; priority {', '.join(cfg.pool.priority)}")
    # A tier's entries name models; whether one checks itself is its subscription's seat's.
    checking = {seat.model for provider in cfg.pool.ordered() for seat in cfg.pool.seats(provider)
                if seat.self_check}
    for tier in providers_mod.TIERS:
        order = ", ".join(e.model + (" (self-check)" if e.model in checking else "")
                          for e in cfg.pool.tiers.get(tier, ()))
        print(f"{tier} tier, tried in this order: {order or 'none'}")
    return 0


def cmd_doctor(cfg: Config, args: argparse.Namespace) -> int:
    errors: list[str] = []
    warnings: list[str] = []
    ok: list[str] = []
    ctx = _ctx(cfg)
    if ctx.trust.problems:
        errors += [f"trust: {p}" for p in ctx.trust.problems]
    if not ctx.trust.entries:
        errors.append("trust: nobody is on .harness/trust.txt")
    else:
        ok.append(f"trust: {', '.join(ctx.trust.logins())}")
    errors += [f"prompts: {p}" for p in check_templates()]
    if cfg.bot_token:
        try:
            who = ctx.gh.viewer().get("login")
            (ok if str(who).lower() == cfg.bot_login.lower() else warnings).append(
                f"BOT_GITHUB_TOKEN belongs to @{who} (expected @{cfg.bot_login})")
        except GitHubError as exc:
            errors.append(f"BOT_GITHUB_TOKEN does not work: {exc}")
    elif cfg.actions_token and args.work:
        ok.append("no GitHub write token here, as intended: the model job never holds one")
    elif cfg.actions_token:
        warnings.append("no BOT_GITHUB_TOKEN: GitHub writes use the Actions token, so comments "
                        "come from github-actions[bot] and a pull request the bot opens does not "
                        "start CI (so auto-merge never fires)")
    else:
        warnings.append("no GitHub token in the environment")
    if args.work:
        provider = cfg.pool.ordered()[0]
        if args.plan:
            planned = json.loads(Path(args.plan).read_text(encoding="utf-8"))
            provider = cfg.pool.get(planned.get("provider")) or provider
        binary = cfg.bin(provider.cli)
        if not shutil.which(binary):
            errors.append(f"`{binary}` ({provider.id}'s CLI) is not on PATH")
        else:
            version = subprocess.run([binary, "--version"], capture_output=True, text=True)
            ok.append(f"{provider.cli}: {(version.stdout or version.stderr).strip()[:80]}")
        if provider.login == "machine":
            ok.append(f"{provider.id} uses the login on this machine")
        elif not cfg.secret_for(provider.secret):
            errors.append(f"{provider.id}'s secret ({provider.secret}) is not in this job")
    try:
        info = ctx.gh.repo_info()
        # Only a token with push access sees this field; a read-only token cannot tell.
        if "allow_auto_merge" in info and not info.get("allow_auto_merge"):
            warnings.append("the repository does not allow auto-merge (setup --repo-settings)")
        required = ctx.gh.required_checks(cfg.default_branch)
        if required is None:
            warnings.append(f"`{cfg.default_branch}` has no branch protection, so auto-merge "
                            "cannot wait for CI (setup --repo-settings)")
        else:
            missing = [c for c in cfg.required_checks if c not in required]
            if missing:
                warnings.append(f"required checks missing on {cfg.default_branch}: {missing}")
            else:
                ok.append("branch protection requires every CI check")
    except GitHubError as exc:
        warnings.append(f"could not read the repository settings: {exc.status}")
    try:
        names = {label.get("name") for label in ctx.gh.list_labels()}
        missing_labels = [n for n in LABELS if n not in names]
        (warnings if missing_labels else ok).append(
            f"labels missing: {missing_labels}" if missing_labels else "labels: all present")
        if ctx.gh.branch_sha(config_mod.STATE_BRANCH) is None:
            warnings.append(f"no `{config_mod.STATE_BRANCH}` branch yet (setup creates it)")
        state = ctx.store.load()
        if state.get("halted"):
            warnings.append(f"halted: {state.get('halt')}")
        for provider in cfg.pool.ordered():
            refusal = providers_mod.refusal(provider, providers_mod.peek_record(
                state, provider.id), ctx.now(), cfg.timezone)
            if refusal:
                warnings.append(f"{provider.id}: {refusal}")
        if ctx.repo_halted():
            warnings.append(".harness/HALT is on main")
    except GitHubError as exc:
        warnings.append(f"could not read labels or state: {exc}")
    for line in ok:
        print(f"ok       {line}")
    for line in warnings:
        print(f"warning  {line}")
    for line in errors:
        print(f"ERROR    {line}")
    return 1 if errors else 0


def cmd_setup(cfg: Config, args: argparse.Namespace) -> int:
    ctx = _ctx(cfg)
    # `LABELS` is the one list (#187): a missing label is created, and one whose colour or
    # description someone changed by hand is brought back to it.
    for name, (color, description) in LABELS.items():
        done = ctx.gh.sync_label(name, color, description)
        if done:
            print(f"{done} label {name}")
    if ctx.store.ensure():
        print(f"created branch {config_mod.STATE_BRANCH}")
    if args.repo_settings:
        ctx.gh.update_repo(allow_auto_merge=True, delete_branch_on_merge=True)
        print("repository: auto-merge allowed, merged branches deleted")
        ctx.gh.set_protection(cfg.default_branch, list(cfg.required_checks))
        print(f"{cfg.default_branch}: requires {len(cfg.required_checks)} checks")
    print("setup done")
    return 0


def cmd_triage(cfg: Config, args: argparse.Namespace) -> int:
    """Triage a new issue or pull request, or one a person called it on (`triage.py`). Every step
    skips quietly on failure: triage is a convenience, never a reason for a red run."""
    if args.step == "gate":
        payload = json.loads(Path(args.payload).read_text(encoding="utf-8"))
        asked = str((payload.get("inputs") or {}).get("number") or "").strip().lstrip("#")
        if asked:  # `workflow_dispatch`: a person called triage on this thread
            try:
                payload = {"issue": _ctx(cfg, write=False).gh.get_issue(int(asked))}
            except (GitHubError, ValueError) as exc:
                _output({"go": "false", "number": ""})
                print(redact(f"triage: skip: #{asked} could not be read: {exc}"))
                return 0
        elif payload.get("action") == "labeled" and (payload.get("issue") or {}).get("number"):
            # #307: a method label (or `bot:approved` on a suggestion) starts triage. A person
            # gets METHOD_WAIT to set a difficulty and a priority, and then the issue is read
            # again, so their labels count.
            if not triage_mod.starts_triage(str((payload.get("label") or {}).get("name") or "")):
                _output({"go": "false", "number": ""})
                print("triage: skip: not a method label")
                return 0
            time.sleep(triage_mod.METHOD_WAIT.total_seconds())
            number = int(payload["issue"]["number"])
            try:
                payload = {"issue": _ctx(cfg, write=False).gh.get_issue(number)}
            except (GitHubError, ValueError) as exc:
                _output({"go": "false", "number": ""})
                print(redact(f"triage: skip: #{number} could not be read: {exc}"))
                return 0
        go, why = triage_mod.gate(payload, Trust.load(cfg.root / TRUST_PATH), cfg.bot_login,
                                  cfg.root, clock_now(), cfg.timezone, asked=bool(asked))
        thread, _ = triage_mod.thread_of(payload)
        _output({"go": str(go).lower(), "number": thread.get("number") or ""})
        print(f"triage: {'go' if go else 'skip'}: {why}")
        return 0
    number = int(args.number)
    out = Path(args.verdict)
    if args.step == "classify":
        try:
            ctx = _ctx(cfg, write=False)
            thread = ctx.gh.get_issue(number)
            text = triage_mod.prompt(thread, "pull_request" in thread, ctx.gh.list_labels(),
                                     triage_mod.conventions_text(cfg.root),
                                     triage_mod.issue_types(ctx.gh),
                                     ctx.gh.list_issues(state="open", limit=300),
                                     method=triage_mod.method_of(thread))
            model, effort = triage_mod.classifier_model(cfg.root)
            answer = triage_mod.run_muse(cfg.bin("muse"), model, effort, text)
            verdict = triage_mod.parse(answer)
        except Exception as exc:  # noqa: BLE001 - any failure skips
            print(redact(f"triage: {triage_mod.CLASSIFIER} did not classify #{number}: {exc}"))
            return 0
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(json.dumps({"number": number, "verdict": verdict}), encoding="utf-8")
        print(f"triage: #{number}: {json.dumps(verdict)[:500]}")
        return 0
    try:
        written = json.loads(out.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        # No answer from the classifier: the method label's own labels and assignees, and the
        # links the issue's own text and title give, are still made.
        written = {"number": number, "verdict": None}
    if int(written.get("number") or 0) != number:
        print(f"triage: the answer is for #{written.get('number')}, not #{number}")
        return 0
    try:
        ctx = _ctx(cfg)
        thread = ctx.gh.get_issue(number)
        is_pr = "pull_request" in thread
        repo_labels = {str(label.get("name")) for label in ctx.gh.list_labels()}
        open_issues: dict[int, dict] | None = None
        linked: dict[str, Any] = {}
        if not is_pr:
            open_issues = {int(i["number"]): i for i in ctx.gh.list_issues(state="open", limit=300)
                           if "pull_request" not in i}
            linked = {"blocked_by": {int(i["number"]) for i in ctx.gh.blocked_by(number)},
                      "blocking": {int(i["number"]) for i in ctx.gh.blocking(number)},
                      "parent": triage_mod.parent_number(thread)}
        plan = triage_mod.decide(written.get("verdict"), thread, is_pr, repo_labels,
                                 cfg.bot_login, triage_mod.issue_types(ctx.gh), open_issues,
                                 linked, asked=bool(getattr(args, "asked", False)))
        ids = {n: int(i.get("id") or 0) for n, i in (open_issues or {}).items()}
        ids[number] = int(thread.get("id") or 0)
        done = triage_mod.apply(ctx.gh, number, plan, ids)
    except GitHubError as exc:
        print(redact(f"triage: could not triage #{number}: {exc}"))
        return 0
    if plan.classified:
        # Every thread triage classified says so (a closed one, or an issue whose method label
        # went before the answer came, was not classified, and gets nothing).
        try:
            ctx.gh.create_comment(number, triage_mod.comment(
                plan, done, is_pr=is_pr, answered=isinstance(written.get("verdict"), dict)))
        except GitHubError as exc:
            print(redact(f"triage: could not comment on #{number}: {exc}"))
    lines = done or ["nothing to change"]
    report = f"Triage of #{number}:\n" + _bullets(lines + plan.notes)
    print(report)
    _summary(report)
    return 0


def cmd_forget(cfg: Config, args: argparse.Namespace) -> int:
    """Clear an item's failure count and findings, so it can be queued fresh."""
    ctx = _ctx(cfg)
    number = int(args.number)
    ctx.store.update(lambda s: state_item(s, number).update(failures=0, last_findings=[]),
                     f"forget #{number}")
    print(f"#{number}: failures and findings cleared")
    return 0


def parser() -> argparse.ArgumentParser:
    top = argparse.ArgumentParser(prog="python -m harness", description=__doc__)
    sub = top.add_subparsers(dest="command", required=True)
    p = sub.add_parser("plan", help="decide what this night run does, and claim it")
    p.add_argument("--out", required=True)
    p.add_argument("--force", action="store_true", help="ignore the subscriptions' hours")
    p.add_argument("--quiet-ok", default=None,
                   help="the gate's verdict: the shared subscription it saw quiet, or empty")
    p.add_argument("--item", default="")
    p.add_argument("--mode", default="auto", choices=plan_mod.MODES)
    p = sub.add_parser("peek", help="would a run find work? changes nothing")
    p.add_argument("--force", action="store_true", help="ignore the subscriptions' hours")
    p.add_argument("--item", default="")
    p.add_argument("--mode", default="auto", choices=plan_mod.MODES)
    sub.add_parser("quiet", help="wait until nobody else is spending the subscription")
    p = sub.add_parser("work", help="the model job: build, review and fix one item")
    p.add_argument("--plan", required=True)
    p.add_argument("--out", required=True)
    p.add_argument("--work-dir", required=True)
    p = sub.add_parser("deliver", help="check the model job's output and publish it")
    p.add_argument("--plan", required=True)
    p.add_argument("--out", required=True)
    p.add_argument("--action", default="", help="the plan job's action output, which wins")
    p.add_argument("--number", default="", help="the plan job's number output, which wins")
    p.add_argument("--provider", default="", help="the plan job's provider output, which wins")
    p = sub.add_parser("event", help="handle one GitHub event")
    p.add_argument("--name", default="")
    p.add_argument("--payload", default="")
    sub.add_parser("sweep", help="answer any request an event handler never answered")
    sub.add_parser("status", help="print the status report")
    dashboard = sub.add_parser("dashboard", help="rewrite the pinned status issue (bot-status "
                               "runs it every ten minutes; each sweep runs it once)")
    dashboard.add_argument("--every", type=int, default=0,
                           help="seconds between rewrites; 0 rewrites it once")
    dashboard.add_argument("--for", dest="for_seconds", type=int, default=0,
                           help="how long to keep rewriting it, in seconds")
    dashboard.add_argument("--sweep", action="store_true",
                           help="sweep before each rewrite, as the ten-minute sweep does")
    dashboard.add_argument("--stats", action="store_true",
                           help="also rewrite the pinned statistics issue, every 30 minutes")
    dashboard.add_argument("--companions", action="store_true",
                           help="also sweep and draw each other bot (Squishy) in a process of "
                           "its own, and put its section in the issue")
    dashboard.add_argument("--section-only", action="store_true",
                           help="print this bot's section of the status issue and write nothing "
                           "(the night bot's loop runs this as Squishy)")
    p = sub.add_parser("stats", help="rewrite the pinned statistics issue now")
    p.add_argument("--force", action="store_true", help="even if it was rewritten under an hour ago")
    p.add_argument("--companions", action="store_true",
                   help="also put in each other bot's section (Squishy's), drawn by its own process")
    p.add_argument("--section-only", action="store_true",
                   help="print this bot's section of the statistics issue and write nothing")
    p = sub.add_parser("halt", help="stop all model work")
    p.add_argument("reason", nargs="*")
    sub.add_parser("start", help="lift a halt")
    p = sub.add_parser("dispatch", help="start a night run now")
    p.add_argument("--item", default="")
    p.add_argument("--force", action="store_true")
    p.add_argument("--mode", default="auto", choices=plan_mod.MODES)
    for name in ("providers", "window"):
        p = sub.add_parser(name, help="each subscription's hours, limits and readiness")
        p.add_argument("--offline", action="store_true", help="do not read the state file")
    p = sub.add_parser("doctor", help="check the configuration and the repository")
    p.add_argument("--work", action="store_true", help="also check the model job's CLI and secret")
    p.add_argument("--plan", default="", help="the plan file, for the provider to check")
    p = sub.add_parser("setup", help="create or update the labels, and create the state branch")
    p.add_argument("--repo-settings", action="store_true",
                   help="also allow auto-merge and protect the default branch (needs admin)")
    p = sub.add_parser("triage", help="label, assign, title and link a new issue or pull "
                       "request, or one a person called it on")
    p.add_argument("step", choices=("gate", "classify", "apply"))
    p.add_argument("--payload", default="", help="gate: the event's payload file")
    p.add_argument("--number", default="0", help="classify, apply: the issue or pull request")
    p.add_argument("--verdict", default="", help="classify writes it, apply reads it")
    p.add_argument("--asked", action="store_true",
                   help="apply: a person called triage on it, so an issue with no method label "
                   "is classified all the same")
    p = sub.add_parser("forget", help="clear an item's failure count")
    p.add_argument("number")
    return top


COMMANDS = {
    "peek": cmd_peek,
    "quiet": cmd_quiet,
    "plan": cmd_plan,
    "work": cmd_work,
    "deliver": cmd_deliver,
    "event": cmd_event,
    "sweep": cmd_sweep,
    "status": cmd_status,
    "dashboard": cmd_dashboard,
    "stats": cmd_stats,
    "halt": cmd_halt,
    "start": cmd_start,
    "dispatch": cmd_dispatch,
    "providers": cmd_providers,
    "window": cmd_providers,
    "doctor": cmd_doctor,
    "setup": cmd_setup,
    "forget": cmd_forget,
    "triage": cmd_triage,
}


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    try:
        cfg = config_mod.load()
        return COMMANDS[args.command](cfg, args)
    except (ConfigError, HarnessError) as exc:
        print(redact(f"error: {exc}"), file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
