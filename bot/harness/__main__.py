"""The command line: `python -m harness <command>` from the `bot/` directory.

The workflows call `plan`, `work`, `deliver` and `event`. An operator calls `status`, `halt`,
`start`, `dispatch`, `doctor`, `setup` and `window`, with a token in BOT_GITHUB_TOKEN,
GITHUB_TOKEN or GH_TOKEN, or a logged-in `gh`.
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any

from harness import config as config_mod
from harness import context as context_mod
from harness import deliver as deliver_mod
from harness import events as events_mod
from harness import plan as plan_mod
from harness import status as status_mod
from harness.clock import human_delta, iso
from harness.config import LABELS, Config
from harness.errors import ConfigError, GitHubError, HarnessError
from harness.redact import redact
from harness.runner import get_runner
from harness.state import record_usage, usage_refusal
from harness.state import item as state_item
from harness.work import Worker, check_templates


def _local_token() -> str:
    """A logged-in `gh`'s token, for an operator running this by hand."""
    gh = shutil.which("gh")
    if not gh:
        return ""
    proc = subprocess.run([gh, "auth", "token"], capture_output=True, text=True, check=False)
    return proc.stdout.strip() if proc.returncode == 0 else ""


def _ctx(cfg: Config, *, write: bool = True) -> context_mod.Context:
    token = cfg.write_token if write else (cfg.actions_token or cfg.bot_token)
    if not token:
        token = _local_token()
    return context_mod.build(cfg, token=token)


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


def _dump(obj: Any) -> None:
    print(redact(json.dumps(obj, indent=2, default=str)))


# ---------------------------------------------------------------------- workflow commands


def cmd_plan(cfg: Config, args: argparse.Namespace) -> int:
    ctx = _ctx(cfg)
    item = int(args.item) if str(args.item or "").strip().lstrip("#").isdigit() else None
    planned = plan_mod.make(ctx, force=args.force, item=item, mode=args.mode)
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(planned, indent=2) + "\n", encoding="utf-8")
    _output({"action": planned["action"], "number": planned.get("number") or ""})
    what = planned["action"] if planned["action"] == "none" else (
        f"{planned['action']} #{planned.get('number')}" if planned.get("number") else planned["action"])
    _summary(f"### Plan: {what}\n\n{planned.get('reason', '')}\n")
    print(f"plan: {what} {planned.get('reason', '')}".strip())
    return 0


def make_probe(ctx: context_mod.Context, number: int | None):
    def probe(last_usage: dict | None):
        if ctx.repo_halted():
            return ("halted by .harness/HALT on main", "halt")
        state = ctx.store.load()
        if state.get("halted"):
            return ("halted by /harness halt", "halt")
        if number is not None:
            record = state["items"].get(str(number), {})
            if record.get("stop_requested"):
                return (f"stopped by @{record.get('stopped_by', 'someone')}", "stop")
        record_usage(state, last_usage, None, ctx.now())
        refusal = usage_refusal(state, dict(ctx.cfg.usage_stop), ctx.now())
        if refusal:
            return (f"usage stop: {refusal}", "usage")
        return None
    return probe


def cmd_work(cfg: Config, args: argparse.Namespace) -> int:
    planned = json.loads(Path(args.plan).read_text(encoding="utf-8"))
    ctx = _ctx(cfg, write=False)
    number = planned.get("number")
    worker = Worker(
        cfg, planned, get_runner(cfg), cfg.root, Path(args.work_dir), Path(args.out),
        probe=make_probe(ctx, int(number) if number else None),
    )
    result = worker.run()
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
                                    action=args.action or None, number=number).run()
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


def cmd_status(cfg: Config, args: argparse.Namespace) -> int:
    print(status_mod.report(_ctx(cfg)))
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


def cmd_window(cfg: Config, args: argparse.Namespace) -> int:
    ctx = _ctx(cfg, write=False)
    now = ctx.now()
    window = ctx.window
    if window.is_open(now):
        closes = window.closes_at(now)
        print(f"open ({window.describe()}); closes in {human_delta(closes - now) if closes else '?'}")
    else:
        print(f"closed ({window.describe()}); opens in {human_delta(window.next_open(now) - now)}")
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
    elif cfg.actions_token:
        warnings.append("no BOT_GITHUB_TOKEN: GitHub writes use the Actions token, so comments "
                        "come from github-actions[bot] and a pull request the bot opens does not "
                        "start CI (so auto-merge never fires)")
    else:
        warnings.append("no GitHub token in the environment")
    if args.work:
        if not shutil.which(cfg.claude_bin):
            errors.append(f"`{cfg.claude_bin}` is not on PATH")
        else:
            version = subprocess.run([cfg.claude_bin, "--version"], capture_output=True, text=True)
            ok.append(f"claude: {version.stdout.strip()}")
        if not cfg.claude_token_present:
            errors.append("CLAUDE_CODE_OAUTH_TOKEN is not set")
    try:
        info = ctx.gh.repo_info()
        if not info.get("allow_auto_merge"):
            warnings.append("the repository does not allow auto-merge (setup --repo-settings)")
        protection = ctx.gh.get_protection(cfg.default_branch)
        if protection is None:
            warnings.append(f"`{cfg.default_branch}` has no branch protection, so auto-merge "
                            "cannot wait for CI (setup --repo-settings)")
        else:
            have = set((protection.get("required_status_checks") or {}).get("contexts") or [])
            missing = [c for c in cfg.required_checks if c not in have]
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
        refusal = usage_refusal(state, dict(cfg.usage_stop), ctx.now())
        if refusal:
            warnings.append(f"usage stop: {refusal}")
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
    for name, (color, description) in LABELS.items():
        if ctx.gh.ensure_label(name, color, description):
            print(f"created label {name}")
    if ctx.store.ensure():
        print(f"created branch {config_mod.STATE_BRANCH}")
    if args.repo_settings:
        ctx.gh.update_repo(allow_auto_merge=True, delete_branch_on_merge=True)
        print("repository: auto-merge allowed, merged branches deleted")
        ctx.gh.set_protection(cfg.default_branch, list(cfg.required_checks))
        print(f"{cfg.default_branch}: requires {len(cfg.required_checks)} checks")
    print("setup done")
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
    p.add_argument("--force", action="store_true", help="ignore the night window")
    p.add_argument("--item", default="")
    p.add_argument("--mode", default="auto", choices=plan_mod.MODES)
    p = sub.add_parser("work", help="the model job: build, review and fix one item")
    p.add_argument("--plan", required=True)
    p.add_argument("--out", required=True)
    p.add_argument("--work-dir", required=True)
    p = sub.add_parser("deliver", help="check the model job's output and publish it")
    p.add_argument("--plan", required=True)
    p.add_argument("--out", required=True)
    p.add_argument("--action", default="", help="the plan job's action output, which wins")
    p.add_argument("--number", default="", help="the plan job's number output, which wins")
    p = sub.add_parser("event", help="handle one GitHub event")
    p.add_argument("--name", default="")
    p.add_argument("--payload", default="")
    sub.add_parser("status", help="print the status report")
    p = sub.add_parser("halt", help="stop all model work")
    p.add_argument("reason", nargs="*")
    sub.add_parser("start", help="lift a halt")
    p = sub.add_parser("dispatch", help="start a night run now")
    p.add_argument("--item", default="")
    p.add_argument("--force", action="store_true")
    p.add_argument("--mode", default="auto", choices=plan_mod.MODES)
    sub.add_parser("window", help="is the night window open")
    p = sub.add_parser("doctor", help="check the configuration and the repository")
    p.add_argument("--work", action="store_true", help="also check the claude CLI and its token")
    p = sub.add_parser("setup", help="create labels and the state branch")
    p.add_argument("--repo-settings", action="store_true",
                   help="also allow auto-merge and protect the default branch (needs admin)")
    p = sub.add_parser("forget", help="clear an item's failure count")
    p.add_argument("number")
    return top


COMMANDS = {
    "plan": cmd_plan,
    "work": cmd_work,
    "deliver": cmd_deliver,
    "event": cmd_event,
    "status": cmd_status,
    "halt": cmd_halt,
    "start": cmd_start,
    "dispatch": cmd_dispatch,
    "window": cmd_window,
    "doctor": cmd_doctor,
    "setup": cmd_setup,
    "forget": cmd_forget,
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
