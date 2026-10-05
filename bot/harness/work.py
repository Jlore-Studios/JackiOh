"""The model job: one item through builder -> checks -> adversarial review, a review run of a bot
pull request, a planning run, or a suggestion survey, all on the one subscription `plan` chose.

Each role runs on the seat `plan` gave it (`plan["seats"]`: a model and its tier on this
subscription): a planning session first when the item has no plan yet, the builder and its
fixes, and the run's own adversarial reviewer. A builder whose seat has `self_check` (Devin) checks
its own change before any review: build, checks, a self check by a fresh session of the same
model, and on findings a fix and the checks and the self check again, up to
`max_self_check_rounds`. A clean self check is never an approval. A run whose subscription has no
seat of at least medium to review in the run (only weak ones) ends with the change built, for a
review run to judge; a weak model reviews only in a review run, and only an easy item.

This job holds no GitHub write credential. It writes `result.json` and a git bundle of the branch
to the output directory; the deliver job checks the bundle itself and pushes it. Prompts are read
into memory when the job starts, so nothing the model writes to disk can change what a later
call is asked.
"""

from __future__ import annotations

import json
import os
from string import Template as string_template
from datetime import datetime, timedelta
from pathlib import Path
from typing import Any, Callable

from harness import disk as disk_mod
from harness import easy as easy_mod
from harness import gates as gates_mod
from harness import prompts, review_rule, verdicts
from harness import providers as providers_mod
from harness.clock import iso, now as clock_now
from harness.config import MIN_TIER, PLAN_FLOOR, Config, child_env
from harness.git import MARKER_LINE, Git, Identity, worktree_add
from harness.prompts import data
from harness.providers import hosted
from harness.redact import redact, redact_json
from harness.runner import RunRequest, RunResult
from harness.verdicts import Finding

BUILDER_TOOLS = (
    "Agent", "Task", "Bash", "Read", "Edit", "Write", "MultiEdit", "Glob", "Grep",
    "NotebookEdit", "TodoWrite", "EnterWorktree", "ExitWorktree",
)
READER_TOOLS = ("Agent", "Task", "Bash", "Read", "Glob", "Grep", "TodoWrite")
#: Speed bumps, not a sandbox: a shell can always reach the network another way. The model job
#: holds no GitHub write token, and the deliver job checks everything it publishes.
NETWORK_DENY = tuple(f"Bash({tool}:*)" for tool in (
    "curl", "wget", "nc", "ncat", "netcat", "ssh", "scp", "sftp", "rsync", "telnet", "gh",
    "git push", "git remote",
))
BUILDER_DENY = ("WebFetch", "WebSearch") + NETWORK_DENY
READER_DENY = BUILDER_DENY + ("Edit", "Write", "MultiEdit", "NotebookEdit", "Bash(git commit:*)")

DIFF_IN_PROMPT = 60_000
#: A call that fails sooner than this with nothing to show (no text) is the subscription's failure,
#: not the item's: its CLI or login is broken (#317 part 1). Devin failed every call in seconds
#: from 2026-10-05 07:30Z, and each one was charged to its item as a build or an unreadable review.
INSTANT_FAILURE_S = 60
#: The builder's running notes, at the top of the worktree. Git ignores it (`info/exclude`), so it
#: is never committed; the harness reads it when the run ends and hands it to the next agent.
NOTES_FILE = ".bot-notes.md"
NOTES_CHARS = 8000
NOTES_ASK = f"""

## Keep notes for whoever picks this up

Keep a short running log in `{NOTES_FILE}` at the top of this worktree (git ignores it, so it is
never delivered): your plan, what is done, what is next, the decisions you made and why, and the
dead ends you hit. Update it as you go, not only at the end. Your session can be cut off at any
moment (a usage limit, the clock), and the next agent, possibly another model, starts from this
file and the branch."""
#: How much of a planner's answer becomes the plan: its beginning, where the goal and the first
#: steps are, never its end alone.
PLAN_CHARS = 20_000
PLAN_CUT = "\n\n…(the plan was cut here; the planner wrote more)"
SELF_CHECK_CONTEXT = """This is a self check, not a review. You are the same model that built
this change, in a fresh session, and nothing you say here approves it: an independent reviewer
judges it afterwards. Adversarially find every reason your own change should not ship, as that
reviewer would, and report each as a blocking finding. Self check {n} of at most {cap}."""
SETTINGS_FILES = (".claude/settings.json", ".claude/settings.local.json", ".mcp.json")
MANIFESTS = ("package.json", "pnpm-lock.yaml", "pnpm-workspace.yaml", ".npmrc")

#: How an interruption ends the item: `stop` and `infra` have statuses of their own; the rest
#: (`budget`, `usage`, `halt`, and `disk` for the machine's disk filling up during the work) are
#: `interrupted` and requeue.
KIND_STATUS = {"stop": "stopped", "infra": "infra"}


class Interrupt(Exception):
    """Stop the item now and hand what exists to the deliver job."""

    def __init__(self, reason: str, kind: str = "budget", reset_at: str | None = None) -> None:
        super().__init__(reason)
        self.reason = reason
        self.kind = kind
        self.reset_at = reset_at


#: What a probe returns when the run must stop: the reason, and its kind (`halt`, `stop` for a
#: person stopping this item, or `usage`).
Probe = Callable[[dict | None], "tuple[str, str] | None"]


def _is_manifest(path: str) -> bool:
    return path.rsplit("/", 1)[-1] in MANIFESTS


class Worker:
    def __init__(
        self,
        cfg: Config,
        plan: dict[str, Any],
        runner: Any,
        repo_dir: Path,
        work_dir: Path,
        out_dir: Path,
        *,
        probe: Probe | None = None,
        after_call: Callable[[], None] | None = None,
        now: Callable[[], datetime] | None = None,
        env: dict[str, str] | None = None,
        disk_reader: Callable[[], dict[str, Any]] | None = None,
    ) -> None:
        self.cfg = cfg
        self.plan = plan
        self.runner = runner
        self.repo = Git(repo_dir, env)
        self.work_dir = Path(work_dir)
        self.out_dir = Path(out_dir)
        self.probe = probe
        #: Run after every model call: keeps a login the CLI just refreshed (`logins.seal`), so
        #: a job that is killed later still hands it on.
        self.after_call = after_call
        self.now = now or (lambda: clock_now(cfg.now_override))
        self.env = dict(env) if env is not None else child_env()
        self.started = self.now()
        self.deadline = self.started + timedelta(minutes=cfg.job_budget_minutes)
        self.templates = {name: prompts.load(name) for name in prompts.NAMES}
        self.system = self.templates["system"].substitute(bot=cfg.bot_login, repo=cfg.repo)
        self.provider = cfg.pool.get(plan.get("provider")) or cfg.pool.ordered()[0]
        #: The checks this run runs. On the bot's machine, only those marked for it (`Gate.machine`):
        #: every job there shares two vCPUs, and CI on the pull request runs the rest anyway.
        self.on_machine = not hosted(str(plan.get("runs_on") or self.provider.runs_on))
        self.gates = [gate for gate in cfg.gates if gate.machine or not self.on_machine]
        #: How full the disk is (`disk.reading`), read on the machine only: its subscriptions share
        #: one disk, and a full one fails whichever job writes next.
        self.disk_reader = disk_reader or (lambda: disk_mod.reading(
            self.work_dir if self.work_dir.exists() else cfg.root, self.now()))
        main = cfg.pool.seats(self.provider)[0]
        seats = plan.get("seats") if isinstance(plan.get("seats"), dict) else None
        #: The model each role runs on. A plan from before seats runs every role on the
        #: subscription's main model, its reviewer included.
        self.build_seat = self._seat(seats, "build") or main
        self.review_seat = self._seat(seats, "review") if seats is not None else main
        self.plan_seat = self._seat(seats, "plan")
        self.self_check = bool(seats.get("self_check")) if seats is not None else False
        self.main_seat = main
        self.plan_text = ""
        self.minutes = 0.0
        self.build_transcript: Path | None = None
        self.who = Identity.bot(cfg.bot_login, cfg.bot_user_id)
        self.last_usage: dict | None = None
        #: A fresh usage reading taken just before the run (`cmd_work`'s ping), if any.
        self.start_usage: dict | None = None
        self.calls = 0
        self.wt: Git | None = None
        self.base_ref = f"origin/{cfg.default_branch}"
        self.base_sha = ""
        self.start_sha = ""
        self.installed_at: str | None = None
        self.approved_sha: str | None = None
        self._base_wt: Git | None = None
        self._base_gate_cache: dict[str, bool] = {}
        #: The forbidden paths the last path guard put back (`_guard`).
        self.put_back: list[str] = []
        #: The last cut-off revision's work this run starts from (`bot/wip/<pr>`), if any.
        self.wip_used = ""
        #: For a conflict revision of a cleared change: what its builder and its reviewer are told
        #: (`_cleared_notes`).
        self.carry_build = ""
        self.carry_review = ""
        self.result: dict[str, Any] = {
            "version": 1,
            "action": plan.get("action"),
            "number": plan.get("number"),
            "status": "failed",
            "reason": "",
            "started_at": iso(self.started),
            "cycles": [],
            "provider": self.provider.id,
            "family": self.provider.family,
            "seats": {"plan": self.plan_seat.to_dict() if self.plan_seat else None,
                      "build": self.build_seat.to_dict(),
                      "review": self.review_seat.to_dict() if self.review_seat else None,
                      "self_check": self.self_check},
        }

    def _seat(self, seats: dict[str, Any] | None, role: str) -> Any:
        """The seat `plan` gave `role`, if it is one this subscription has."""
        entry = (seats or {}).get(role)
        if not isinstance(entry, dict):
            return None
        return self.cfg.pool.seat(self.provider.id, str(entry.get("model") or ""))

    def seat_for(self, role: str) -> Any:
        if role == "plan":
            return self.plan_seat or self.main_seat
        if role == "review":
            return self.review_seat or self.build_seat
        if role == "suggest":
            return self.main_seat
        return self.build_seat  # build, fix, revise, and the builder's own self check

    # ------------------------------------------------------------------ plumbing

    def seconds_left(self) -> float:
        return (self.deadline - self.now()).total_seconds()

    def check(self) -> None:
        """Raise Interrupt when a halt, a stop, the usage stop or the clock says to stop."""
        if self.seconds_left() < self.cfg.min_minutes_for_a_call * 60:
            raise Interrupt("the run's time budget is spent", "budget")
        if self.probe is not None:
            found = self.probe(self.last_usage)
            if found:
                reason, kind = found
                raise Interrupt(reason, kind)

    def render(self, name: str, **values: Any) -> str:
        return self.templates[name].substitute({k: str(v) for k, v in values.items()})

    def _clear_settings(self, cwd: Path) -> None:
        """Delete untracked Claude settings in the worktree, so no call inherits a hook or an MCP
        server an earlier call planted."""
        for name in SETTINGS_FILES:
            target = cwd / name
            if not target.exists():
                continue
            tracked = Git(cwd, self.env).run("ls-files", "--error-unmatch", name, check=False)
            if tracked.returncode != 0:
                target.unlink()

    def call(self, role: str, prompt: str, cwd: Path, *, reader: bool) -> RunResult:
        self.calls += 1
        self._clear_settings(cwd)
        timeout = int(min(self.cfg.call_timeout_minutes * 60, self.seconds_left() - 300))
        where = self.out_dir if self.cfg.upload_transcripts else self.work_dir
        transcript = where / "transcripts" / f"{self.calls:02d}-{role}.jsonl"
        if not reader:
            prompt += NOTES_ASK
            self.build_transcript = transcript
        seat = self.seat_for(role)
        request = RunRequest(
            role=role,
            prompt=prompt,
            cwd=cwd,
            system_append=self.system,
            allowed_tools=READER_TOOLS if reader else BUILDER_TOOLS,
            disallowed_tools=READER_DENY if reader else BUILDER_DENY,
            max_turns=int(self.cfg.max_turns.get(role) or self.cfg.max_turns["review"]),
            timeout_s=max(60, timeout),
            model=seat.model,
            effort=(self.provider.fix_effort if role == "fix" and self.provider.fix_effort
                    else seat.effort),
            transcript=transcript,
            read_only=reader,
            extra_dirs=self._git_dirs(cwd),
            usage_stop=self._usage_stop if self.provider.limits.stops else None,
        )
        result = self.runner.run(request)
        if self.after_call is not None:
            self.after_call()
        self.minutes += result.duration_s / 60
        if result.usage:
            self.last_usage = result.usage
        if result.extra.get("usage_stop"):
            raise Interrupt(f"`{self.provider.id}` stops mid-call: {result.extra['usage_stop']}",
                            "usage")
        if result.rate_limited:
            raise Interrupt(f"`{self.provider.id}` reached its usage limit", "usage",
                            result.reset_at)
        if result.infra:
            raise Interrupt(f"the {self.provider.cli} CLI could not run: {result.error}", "infra")
        if (not result.ok and not result.timed_out and not (result.text or "").strip()
                and result.duration_s < INSTANT_FAILURE_S):
            raise Interrupt(f"the {self.provider.cli} CLI failed at once: "
                            f"{result.error or 'it said nothing'}", "infra")
        return result

    def _usage_stop(self, usage: dict[str, Any]) -> str | None:
        """While a call runs: the reading it just streamed, against the provider's caps (and
        its `off_hours` ones outside its window). A reason stops the call there."""
        now = self.now()
        entry = {"usage": {**usage, "observed_at": iso(now)}}
        return providers_mod.refusal(self.provider, entry, now, self.cfg.timezone)

    def _start_check(self) -> None:
        """The fresh reading taken just before the run: a run over its cap, or a build or a
        revision without `start_headroom` under it, ends here before any model work. Without it a
        run started from whatever reading the last run left, none at all after a reset."""
        if not self.start_usage:
            return
        self.last_usage = self.start_usage
        now = self.now()
        entry = {"usage": {**self.start_usage, "observed_at": iso(now)}}
        reason = providers_mod.refusal(self.provider, entry, now, self.cfg.timezone,
                                       starting=self.plan.get("action") in ("build", "revise"))
        if reason:
            raise Interrupt(f"`{self.provider.id}` stops before it starts: {reason}", "usage")

    def _git_dirs(self, cwd: Path) -> tuple[str, ...]:
        """The repository's git directory, which a worktree's git commands write to and a
        sandboxed CLI must be told about."""
        proc = Git(cwd, self.env).run("rev-parse", "--path-format=absolute", "--git-common-dir",
                                      check=False)
        found = proc.stdout.strip() if proc.returncode == 0 else ""
        return (found,) if found else ()

    def write_result(self) -> Path:
        self.out_dir.mkdir(parents=True, exist_ok=True)
        self.result["usage"] = self.last_usage
        self.result["finished_at"] = iso(self.now())
        self.result["model_calls"] = self.calls
        self.result["minutes"] = round(self.minutes, 1)
        handoff = self._handoff()
        if handoff:
            self.result["handoff"] = handoff
        path = self.out_dir / "result.json"
        path.write_text(json.dumps(redact_json(self.result), indent=2) + "\n", encoding="utf-8")
        return path

    # ------------------------------------------------------------------ entry

    def run(self) -> dict[str, Any]:
        self.out_dir.mkdir(parents=True, exist_ok=True)
        try:
            if self.on_machine:
                self._disk_check()
            self._start_check()
            action = self.plan.get("action")
            if action == "suggest":
                self._suggest()
            elif action == "plan":
                self._plan_run()
            elif action in ("build", "revise"):
                self._item()
            elif action == "review":
                self._second_review()
            else:
                self.result.update(status="nothing", reason="the plan had nothing to do")
        except Interrupt as stop:
            self._interrupted(stop)
        except Exception as exc:  # noqa: BLE001 - every failure must still leave a result
            reason = redact(f"{type(exc).__name__}: {exc}")[:2000]
            if disk_mod.is_full(exc):
                # The machine's disk, not the item: a pause that keeps what was built.
                self._interrupted(Interrupt(f"the machine's disk filled up ({reason})", "disk"))
            else:
                self.result.update(status="failed", reason=reason)
                self._save_wip("the harness failed")
        finally:
            if self.on_machine:
                self._read_disk("end")
            self.write_result()
        return self.result

    def _read_disk(self, when: str) -> dict[str, Any] | None:
        disk = self.result.setdefault("disk", {})
        try:
            found = self.disk_reader()
        except OSError:
            return None
        disk[when] = found
        return found

    def _disk_check(self) -> None:
        """On the machine, before any work: room for this job (`disk.py`). Short of
        `CLEAN_BELOW` free, clean what this user's jobs can do without first; still short of
        `FLOOR`, stop on the machine's account, never the item's."""
        found = self._read_disk("start")
        self.result["disk"]["runner"] = os.environ.get("RUNNER_NAME", "")
        if found is None or found["free"] >= disk_mod.CLEAN_BELOW:
            return
        self.result["disk"]["cleaned"] = disk_mod.clean(self.cfg.root, self.env)
        found = self._read_disk("after_clean") or found
        if found["free"] < disk_mod.FLOOR:
            self.result["infra_scope"] = "machine"
            raise Interrupt(f"the machine's disk is {disk_mod.describe(found)}, under the "
                            f"{disk_mod.size(disk_mod.FLOOR)} a job needs, even after cleaning",
                            "infra")

    # ------------------------------------------------------------------ an item

    def _install(self) -> gates_mod.GateResult:
        assert self.wt is not None
        result = gates_mod.run_gate(self.cfg.install, self.wt.cwd, self.env, int(self.seconds_left()))
        if result.ok:
            self.installed_at = self.wt.head()
        return result

    def _prepare(self, *, merge_main: bool = True, install: bool = True) -> list[str]:
        number = int(self.plan["number"])
        branch = str(self.plan["branch"])
        default = self.cfg.default_branch
        self.repo.run("fetch", "--quiet", "--no-tags", "origin",
                      f"+refs/heads/{default}:refs/remotes/origin/{default}")
        self.repo.run("fetch", "--quiet", "--no-tags", "origin",
                      f"+refs/heads/{branch}:refs/remotes/origin/{branch}", check=False)
        remote = f"origin/{branch}"
        has_remote = self.repo.rev(remote) is not None
        if self.plan["action"] in ("revise", "review") and not has_remote:
            raise RuntimeError(f"the pull request's branch {branch} is not on origin")
        start = remote if has_remote else self.base_ref
        self.base_sha = self.repo.rev(self.base_ref) or ""
        self.start_sha = self.repo.rev(start) or ""
        begin = self._wip_start(number, start) if self.plan["action"] == "revise" else start
        self.wt = worktree_add(self.repo, self.work_dir / f"item-{number}", branch, begin)
        self._exclude_notes()
        self._seed_notes()
        self.result.update(branch=branch, base=self.base_sha, start=self.start_sha,
                           remote_branch_existed=has_remote)
        conflicts: list[str] = []
        if has_remote and merge_main:
            conflicts = self.wt.merge(self.base_ref, self.who)
        if any(_is_manifest(p) for p in conflicts):
            return conflicts  # the builder resolves the manifests first; install runs after
        if not install:
            return conflicts
        installed = self._install()
        if not installed.ok and not has_remote:
            self.result["infra_scope"] = "repository"  # not the subscription's fault
            raise Interrupt(f"dependency install failed on untouched main (exit "
                            f"{installed.exit_code}): {installed.tail[-1500:]}", "infra")
        return conflicts

    def _wip_start(self, number: int, start: str) -> str:
        """Where a revision's worktree starts: the last cut-off revision's work on
        `bot/wip/<pr>` (#317 part 3), when it started from the pull request's head as it is now
        and descends from it; otherwise (someone pushed since) the pull request's head. The
        work's `start` stays the pull request's head either way, which deliver checks."""
        wip = self.plan.get("wip")
        if not isinstance(wip, dict) or not wip.get("sha"):
            return start
        ref = f"bot/wip/{int(number)}"
        self.repo.run("fetch", "--quiet", "--no-tags", "origin",
                      f"+refs/heads/{ref}:refs/remotes/origin/{ref}", check=False)
        head = self.repo.rev(f"origin/{ref}")
        if (head and wip.get("start") == self.start_sha and self.repo.run(
                "merge-base", "--is-ancestor", self.start_sha, head, check=False).returncode == 0):
            self.wip_used = ref
            self.result["wip"] = {"used": ref, "sha": head}
            return f"origin/{ref}"
        self.result["wip"] = {"ignored": ref, "why": "the pull request's branch moved since"}
        return start

    def _exclude_notes(self) -> None:
        """Tell git to ignore the notes file in every worktree of this clone."""
        assert self.wt is not None
        common = self.wt.run("rev-parse", "--path-format=absolute", "--git-common-dir",
                             check=False).stdout.strip()
        if not common:
            return
        exclude = Path(common) / "info" / "exclude"
        exclude.parent.mkdir(parents=True, exist_ok=True)
        lines = exclude.read_text(encoding="utf-8").splitlines() if exclude.exists() else []
        if f"/{NOTES_FILE}" not in lines:
            exclude.write_text("\n".join([*lines, f"/{NOTES_FILE}"]) + "\n", encoding="utf-8")

    def _seed_notes(self) -> None:
        """Start the notes file from the handoff's notes (a planner's plan, or an earlier
        builder's notes), so this run's builder keeps them going."""
        assert self.wt is not None
        handoff = self.plan.get("handoff")
        notes = str(handoff.get("notes") or "") if isinstance(handoff, dict) else ""
        if notes.strip():
            (self.wt.cwd / NOTES_FILE).write_text(notes.rstrip() + "\n", encoding="utf-8")

    def _handoff(self) -> dict[str, Any] | None:
        """What the next agent needs if this run did not finish the item: the builder's notes
        and the end of its last session. None when there is neither."""
        if self.wt is None:
            return None
        notes_path = self.wt.cwd / NOTES_FILE
        notes = ""
        if notes_path.is_file():
            notes = notes_path.read_text(encoding="utf-8", errors="replace")
            # A planning run's notes are its plan, which starts at the top; a builder's log is
            # newest at the end.
            notes = notes[:NOTES_CHARS] if self.plan.get("action") == "plan" else notes[-NOTES_CHARS:]
        trail = ""
        trail_of = getattr(self.runner, "trail", None)
        if self.build_transcript is not None and self.build_transcript.exists() and trail_of:
            trail = trail_of(self.build_transcript)
        if not notes.strip() and not trail.strip():
            return None
        return {"provider": self.provider.id, "family": self.provider.family,
                "at": iso(self.now()), "reason": str(self.result.get("reason") or ""),
                "notes": redact(notes), "trail": redact(trail)}

    def _handoff_text(self) -> str:
        """The section a first prompt gets when another agent worked on this before."""
        handoff = self.plan.get("handoff")
        if not isinstance(handoff, dict):
            return ""
        if handoff.get("kind") == "plan":
            if not str(handoff.get("notes") or "").strip():
                return ""
            if self.plan.get("plan_in_issue"):
                return ("\n\n## The plan\n\nThis was planned before anyone built it. "
                        "The plan is the **Plan** section of the issue's description above (a "
                        "person may have edited it since, and that version is the plan), and it is "
                        f"at the top of `{NOTES_FILE}`, which you keep going. Follow it step by "
                        "step unless the code shows it is wrong, and say where you departed from "
                        "it and why.")
            return ("\n\n## The plan\n\nA planning run on "
                    f"`{handoff.get('provider', '?')}` ({handoff.get('family', '?')}) wrote the plan "
                    f"below before anyone built this; it is also at the top of `{NOTES_FILE}`, "
                    "which you keep going. Follow it unless the code shows it is wrong, and say "
                    "where you departed from it and why.\n\n"
                    + data(str(handoff["notes"]), "The plan"))
        where = "in your worktree (see above)" if self.wip_used else "on the branch"
        parts = [f"## Picking up from another agent\n\nAn earlier run on "
                 f"`{handoff.get('provider', '?')}` ({handoff.get('family', '?')}) worked on this "
                 f"and stopped: {handoff.get('reason') or 'no reason recorded'}. Its work so far is "
                 f"{where}. Below are the notes it kept and the end of its session. Check them "
                 "against the diff and re-run the checks before you trust any of it. Go on from "
                 "where it stopped rather than starting over."]
        if str(handoff.get("notes") or "").strip():
            parts.append(data(str(handoff["notes"]), "Its notes"))
        if str(handoff.get("trail") or "").strip():
            parts.append(data(str(handoff["trail"]), "The end of its session"))
        return "\n\n" + "\n\n".join(parts)

    def _wip_text(self) -> str:
        """What a revision is told about the last cut-off run's work (#317 part 3)."""
        if self.wip_used:
            return ("\n\n## Unfinished work from the last run\n\nThe last revision run was cut "
                    "off before it finished. Your worktree starts from `" + self.wip_used + "`, "
                    "which holds its commits (\"bot: work in progress …\") on top of the pull "
                    "request: go on from where it stopped rather than starting over, and check "
                    "what it did against the task before you build on it.")
        found = self.result.get("wip")
        if isinstance(found, dict) and found.get("ignored"):
            return ("\n\n## Unfinished work from the last run\n\nThe last revision run was cut "
                    "off before it finished, but the pull request's branch moved after it "
                    "stopped, so you start from the branch as it is now; that run's commits are "
                    "not in your worktree, and its notes, if any, say what it had done.")
        return ""

    def _branch_state(self) -> str:
        assert self.wt is not None
        log = self.wt.log(self.base_ref)
        if not log:
            return "The branch starts at `main` with no commits of its own yet."
        stat = self.wt.diffstat(self.base_ref)
        return data(f"{log}\n\n{stat}", "Commits on this branch beyond main")

    def _gate_list(self) -> str:
        lines = ["The harness's checks, run in this order after you stop:"]
        lines.append(f"- install: `{self.cfg.install.run}` (again whenever a manifest changed)")
        lines += [f"- {g.name}: `{g.run}`" for g in self.gates]
        lines.append("CI on the pull request also runs the fuzz gate, coverage, the AI gates, "
                     "the Postgres suites and the Cypress e2e specs before anything merges.")
        left = [f"{g.name} (`{g.run}`)" for g in self.cfg.gates if g not in self.gates]
        if left:
            lines.append(
                "This run is on the bot's shared machine, so the harness leaves "
                f"{', '.join(left)} to CI on the pull request, which runs before anything merges. "
                "Don't run whole suites yourself either (`pnpm test`, `pnpm lint`, `pnpm fuzz`, "
                "`pnpm ai:gate`, coverage, e2e): check only what you changed, such as "
                "`pnpm vitest run <test file>` or `pnpm exec eslint <files>`.")
        return "\n".join(lines)

    def _findings_text(self, findings: list[Finding], label: str = "Blocking findings") -> str:
        if not findings:
            return "No blocking findings were recorded."
        body = "\n".join(f.markdown() for f in findings)
        return data(body, label)

    def _first_prompt(self, conflicts: list[str]) -> tuple[str, str]:
        plan = self.plan
        number = plan["number"]
        if plan["action"] == "revise":
            conflict_text = (
                data("\n".join(conflicts), "Files merged with conflict markers")
                if conflicts else "The merge of `main` into the branch was clean."
            ) + self.carry_build
            prompt = self.render(
                "revise",
                number=number, repo=self.cfg.repo, branch=plan["branch"], base=self.base_sha,
                source=plan.get("source", "request"), pull=plan.get("pull", ""),
                issue=plan.get("issue", ""), feedback=plan.get("feedback", ""),
                conflicts=conflict_text, branch_state=self._branch_state(),
                gate_list=self._gate_list(),
            )
            return "revise", prompt + self._wip_text() + self._handoff_text()
        previous = ""
        if plan.get("previous_pr"):
            kept = (f"; its branch is kept as `{plan['previous_branch']}` if you want to look"
                    if plan.get("previous_branch") else "")
            previous = (f"An earlier pull request for this issue, #{plan['previous_pr']}, was "
                        f"closed and this issue built again from `main`: "
                        f"{plan.get('previous_why') or 'it kept failing'}{kept}. Start from "
                        "`main`, learn from what went wrong, and do not repeat it.\n\n")
        prior = plan.get("previous_findings") or []
        if prior:
            previous += ("An earlier run left this unfinished. The last review's blocking "
                         "findings were:\n\n" + self._findings_text([_finding(f) for f in prior]))
        question = str(plan.get("previous_question") or "").strip()
        if question:
            previous += ("\n\nAn earlier run stopped to ask the question below. The answer, if "
                         "someone gave one, is in the comments of the task above.\n\n"
                         + data(question, "The question asked"))
        if conflicts:
            previous += ("\n\n`main` moved since then; merging it left conflict markers in: "
                         + ", ".join(f"`{c}`" for c in conflicts)
                         + ". Resolve them, keeping both sides' meaning. Do not commit.")
        if self.plan_text:
            previous += ("\n\nA planner wrote the plan below for this change before you started; "
                         f"it is also at the top of `{NOTES_FILE}`. Follow it unless the code "
                         "shows it is wrong, and say where you departed from it and why.\n\n"
                         + data(self.plan_text, "The plan"))
        prompt = self.render(
            "build",
            number=number, repo=self.cfg.repo, branch=plan["branch"], base=self.base_sha,
            thread=plan.get("thread", ""), branch_state=self._branch_state(), previous=previous,
            gate_list=self._gate_list(),
        )
        return "build", prompt + self._handoff_text()

    def _cleared_notes(self, conflicts: list[str]) -> tuple[str, str]:
        """What a conflict revision of a cleared change tells its builder and its reviewer, when
        it starts from the commit the review rule cleared: only its own review stands between
        the resolution and `main`. ("", "") for any other run. Deliver decides the carry from
        git alone (`deliver._carry`)."""
        clear = self.plan.get("cleared")
        if (self.plan.get("action") != "revise" or self.plan.get("source") != "conflict"
                or not isinstance(clear, dict) or not self.start_sha
                or clear.get("sha") != self.start_sha):
            return "", ""
        by = f" ({clear['by']})" if clear.get("by") else ""
        cleared_at = f"at `{self.start_sha[:12]}`{by}"
        build = (f"\n\nThe reviews had cleared this change {cleared_at} before `main` moved. "
                 "Resolve the conflicts and change nothing else: if this revision changes only "
                 "the conflicted files and its reviewer approves, it merges without another "
                 "review run. If making it correct takes more (say `main` renamed something the "
                 "change uses), make that change anyway and say so in your report: it then goes "
                 "back to a review run.")
        what = ("the builder resolved the conflicts in " + ", ".join(f"`{c}`" for c in conflicts)
                if conflicts else "the merge was clean")
        review = (f"This revision resolves a conflict on a change the review rule had already "
                  f"cleared {cleared_at}: `main` moved and the branch no longer merged, so the "
                  f"harness merged `main` into it and {what}. If you approve and the revision "
                  "changed nothing beyond those files, it merges with no further review: yours "
                  "is the only review of the resolution, so judge it hardest. In each conflicted "
                  "file both `main`'s change and the branch's change must survive with their "
                  "meaning, nothing `main` brought may be dropped or reverted, and the merged "
                  "whole must still be correct. `git log --merges -1` finds the merge commit, and "
                  "`git show --remerge-diff <it>` shows how each conflict was resolved.")
        return build, review

    def _fix_prompt(self, cycle: int, findings: list[Finding], failures: str,
                    label: str = "Blocking findings") -> str:
        return self.render(
            "fix",
            number=self.plan["number"], repo=self.cfg.repo, branch=self.plan["branch"],
            base=self.base_sha, cycle=cycle, max_cycles=self.cfg.max_review_cycles,
            thread=self.plan.get("thread", ""), branch_state=self._branch_state(),
            findings=self._findings_text(findings, label),
            gate_failures=(data(failures, "Checks this change turned red") if failures
                           else "Every check passed or was already red on main."),
            gate_list=self._gate_list(),
        )

    def _commit(self, message: str) -> bool:
        assert self.wt is not None
        return self.wt.commit_all(message, self.who)

    def _unresolved_markers(self) -> list[str]:
        """Changed files that still hold a conflict marker line."""
        assert self.wt is not None
        changed = [p for p in self.wt.changed_paths(self.base_ref) if (self.wt.cwd / p).is_file()]
        if not changed:
            return []
        proc = self.wt.run("grep", "-l", "-E", MARKER_LINE, "--", *changed, check=False)
        return [p for p in proc.stdout.splitlines() if p.strip()]

    def _anchors(self) -> list[str]:
        """Commits the branch may carry content from: where it started, and the main it merged."""
        return [s for s in (self.start_sha, self.base_sha) if s]

    def _guard(self) -> list[Finding]:
        """Put back forbidden paths the work changed, and report them, conflict markers left
        behind, and an empty change."""
        assert self.wt is not None
        found: list[Finding] = []
        touched = self.wt.unsanctioned("HEAD", self._anchors(), self.cfg.forbidden_paths)
        self.put_back = touched
        if touched:
            self.wt.restore_from(self._anchors(), touched)
            self._commit("bot: put back paths the bot may not change")
            found.append(Finding(
                "blocking", touched[0],
                f"The change edited {', '.join(touched)}, which the bot may not change "
                f"({', '.join(self.cfg.forbidden_paths)}); the harness put them back.",
                "the harness's path guard",
            ))
        markers = self._unresolved_markers()
        if markers:
            found.append(Finding("blocking", markers[0],
                                 f"Conflict markers are still in {', '.join(markers)}.", "git grep"))
        if self.plan["action"] == "build" and not self.wt.changed_paths(self.base_ref):
            found.append(Finding("blocking", "task", "The branch has no change from main.",
                                 "git diff main...HEAD is empty"))
        return found

    def _checks(self) -> list[gates_mod.GateResult]:
        """Install again when a manifest changed (or never succeeded), then every gate."""
        assert self.wt is not None
        results: list[gates_mod.GateResult] = []
        stale = self.installed_at is None or any(
            _is_manifest(p) for p in self.wt.names_between(self.installed_at, "HEAD"))
        if stale:
            install = self._install()
            results.append(install)
            if not install.ok:
                results += [gates_mod.GateResult(g.name, g.run, False, -1, 0.0, "",
                                                 skipped="the install failed")
                            for g in self.gates]
                return results
        results += gates_mod.run_all(self.gates, self.wt.cwd, self.env, self.seconds_left)
        self._mark_pre_existing(results)
        return results

    def _mark_pre_existing(self, results: list[gates_mod.GateResult]) -> None:
        """Run each gate this change left red again on the untouched base. A gate that timed out
        is not: it is inconclusive (`gates.mark_inconclusive`), and on the base it would only
        spend its whole timeout again."""
        gates_mod.mark_inconclusive(results, {g.name for g in self.gates})
        for result in results:
            if (result.ok or result.skipped or result.inconclusive
                    or result.name == self.cfg.install.name):
                continue
            gate = next((g for g in self.gates if g.name == result.name), None)
            if gate is None:
                continue
            if result.name not in self._base_gate_cache:
                base = self._base_worktree()
                if base is None:
                    return
                again = gates_mod.run_gate(gate, base.cwd, self.env, int(self.seconds_left()))
                self._base_gate_cache[result.name] = not again.ok
            result.pre_existing = self._base_gate_cache[result.name]

    def _base_worktree(self) -> Git | None:
        if self._base_wt is None:
            if self.seconds_left() < 20 * 60:
                return None
            path = self.work_dir / "base"
            self._base_wt = worktree_add(self.repo, path, "bot-base-check", self.base_sha)
            install = gates_mod.run_gate(self.cfg.install, path, self.env, int(self.seconds_left()))
            if not install.ok:
                return None
        return self._base_wt

    def _review(self, cycle: int, report: verdicts.BuildReport,
                results: list[gates_mod.GateResult], previous: list[Finding], *,
                context: str = "", max_cycles: int | None = None,
                role: str = "review") -> verdicts.Review:
        assert self.wt is not None
        diff, cut = self.wt.diff(self.base_ref, max_chars=DIFF_IN_PROMPT)
        note = ("The diff below is cut short; run `git diff "
                f"{self.base_sha}...HEAD` for the rest." if cut else
                "The whole diff against the base is below.")
        prompt = self.render(
            "review",
            number=self.plan["number"], repo=self.cfg.repo, cycle=cycle,
            max_cycles=max_cycles or self.cfg.max_review_cycles, branch=self.plan["branch"],
            base=self.base_sha, thread=self.plan.get("thread", ""),
            report=data(report.body or "(the builder wrote no report)", "Builder's report"),
            gates=(gates_mod.table(results) if results else
                   "The harness ran no checks in this run; CI runs every check on the pull "
                   "request before anything merges, and you may run any of them yourself."),
            previous_findings=context or (self._findings_text(previous) if previous
                                          else "This is the first review of this change."),
            diff_note=note, diff=data(diff, "git diff main...HEAD"),
        )
        head = self.wt.head()
        review = verdicts.Review(False, "unreadable")
        for _ in range(2):
            result = self.call(role, prompt, self.wt.cwd, reader=True)
            if self.wt.head() != head:
                self.wt.run("reset", "--quiet", "--hard", head)
            if self.wt.dirty():
                self.wt.discard_worktree_changes()
            review = verdicts.review(result.text)
            if review.readable:
                break
            if not result.ok:
                review.error = redact(str(result.error or "it failed without an error"))[:2000]
            self.check()
        review.reviewed_sha = head
        return review

    def _planning(self) -> None:
        """A planning session before any building: the planner reads the task and the code and
        writes the plan, which the harness puts at the top of the notes file (and so into the
        handoff) for the builder, this run's or a later one's."""
        assert self.wt is not None
        self.check()
        seat = self.seat_for("plan")
        prompt = self.render(
            "plan",
            number=self.plan["number"], repo=self.cfg.repo, branch=self.plan["branch"],
            base=self.base_sha, thread=self.plan.get("thread", ""),
            branch_state=self._branch_state(), gate_list=self._gate_list(),
            difficulty=self.plan.get("difficulty") or "medium",
            rating=self._rating_ask(), easy_rule=self._easy_rule(),
        ) + self._handoff_text()
        head = self.wt.head()
        result = self.call("plan", prompt, self.wt.cwd, reader=True)
        if self.wt.head() != head:
            self.wt.run("reset", "--quiet", "--hard", head)
        if self.wt.dirty():
            self.wt.discard_worktree_changes()
        rated, text = verdicts.rating(redact((result.text or "").strip()))
        if len(text) > PLAN_CHARS:
            text = text[:PLAN_CHARS - len(PLAN_CUT)].rstrip() + PLAN_CUT
        if not result.ok or not text:
            raise RuntimeError(f"the planner on {seat.model} wrote no plan"
                               + (f": {result.error}" if result.error else ""))
        if rated and self._rating_source() == "person":
            rated = None  # a person's label stands; the planner was told not to rate it
        self.plan_text = text
        notes = self.wt.cwd / NOTES_FILE
        earlier = notes.read_text(encoding="utf-8", errors="replace") if notes.is_file() else ""
        notes.write_text(f"# Plan ({seat.model}, {seat.tier})\n\n{text}\n\n# Notes\n\n"
                         f"{earlier}", encoding="utf-8")
        self.result["plan"] = {"seat": seat.to_dict(), "text": text}
        if rated:
            self.result["plan"]["rating"] = rated

    def _rating_source(self) -> str:
        rating = self.plan.get("rating") if isinstance(self.plan.get("rating"), dict) else {}
        return str(rating.get("source") or "")

    def _rating_ask(self) -> str:
        """What the planner is told about rating the item (#317 part 8)."""
        rating = self.plan.get("rating") if isinstance(self.plan.get("rating"), dict) else {}
        difficulty = str(rating.get("difficulty") or self.plan.get("difficulty") or "medium")
        if self._rating_source() == "person":
            return (f"A person set this item's difficulty: `difficulty:{difficulty}`. Plan for "
                    "that, and give no rating line. If the rule below says it is harder, say so "
                    "under **Open questions**.")
        line = ('    <!-- bot: {"difficulty": "easy", "why": "one line: the rule\'s line you were '
                'least sure of, and why it holds"} -->')
        ask = ("Rate how hard it is, by the rule below. The very first line of your answer is a "
               "single HTML comment carrying JSON, with nothing before it, then the plan:\n\n"
               f"{line}\n\n`difficulty` is `easy`, `medium` or `hard`. The rating decides who "
               "builds it: an easy item goes to the weakest model (Devin's SWE-2), which cannot "
               "fill gaps, resolve a hard conflict, or judge what it cannot see.")
        if self._rating_source() == "bot":
            ask += (f" The bot rated it `difficulty:{difficulty}` already"
                    + (f" (`{rating.get('by')}`)" if rating.get("by") else "")
                    + ": you may rate it higher, never lower.")
        return ask

    def _easy_rule(self) -> str:
        rule = self.cfg.easy
        return string_template(easy_mod.RULE).safe_substitute(max_files=rule.max_files,
                                                               max_lines=rule.max_lines)

    def _rated_out(self) -> bool:
        """A build run that planned and rated an unrated item stops after planning when the
        rating needs a stronger model than this run's builder, or a stronger planner than this
        one (`PLAN_FLOOR`): the item goes back to the queue, rated, for those."""
        rated = (self.result.get("plan") or {}).get("rating")
        if not isinstance(rated, dict) or self._rating_source():
            return False
        difficulty = str(rated.get("difficulty"))
        planner = self.seat_for("plan")
        if (providers_mod.tier_at_least(self.build_seat.tier, MIN_TIER[difficulty])
                and providers_mod.tier_at_least(planner.tier, PLAN_FLOOR[difficulty])):
            return False
        self.result.update(status="planned", reason=(
            f"rated difficulty:{difficulty}, which needs a stronger model than this run's to "
            f"{'plan' if not providers_mod.tier_at_least(planner.tier, PLAN_FLOOR[difficulty]) else 'build'} it"))
        return True

    def _plan_run(self) -> None:
        """A planning run: the plan, and nothing built. The deliver job keeps it as the item's
        handoff and queues the item to build from it."""
        self._prepare(merge_main=False, install=False)
        self._planning()
        self.result.update(status="planned", reason=f"planned on {self.seat_for('plan').model}")
        self._finish()

    def _build_pass(self, cycle: int, role: str, prompt: str) -> tuple[Any, dict[str, Any]]:
        """One builder session and its commit. Returns its report, or None when it stopped to
        ask a person (the run then ends as `blocked`), and the round's record."""
        assert self.wt is not None
        before = self.wt.head()
        built = self.call(role, prompt, self.wt.cwd, reader=False)
        report = verdicts.build_report(built.text)
        entry: dict[str, Any] = {
            "n": cycle,
            "builder": {"role": role, "ok": built.ok, "turns": built.turns,
                        "minutes": round(built.duration_s / 60, 1), "status": report.status,
                        "error": built.error, "timed_out": built.timed_out,
                        "model": self.build_seat.model, "tier": self.build_seat.tier},
        }
        if (not built.ok and not built.timed_out and self.wt.head() == before
                and not self.wt.dirty()):
            # A session that failed and changed nothing is no pass (#317 part 1): the checks and
            # the self check would only judge the branch as it was. One that failed after doing
            # work (Claude's turn limit, say) keeps it, and the checks and the review judge it.
            entry["builder"]["changed"] = False
            self.result.update(status="failed", reason=redact(
                f"the builder's session failed and changed nothing: "
                f"{built.error or 'no error given'}")[:2000])
            self._finish()
            return None, entry
        if report.status == "blocked":
            self._save_wip("the builder needs a decision")
            self.result.update(status="blocked", question=report.question, report=report.body,
                               reason="the builder needs a person to decide something")
            self._finish()
            return None, entry
        self._commit(f"bot: {role} pass {cycle} for #{self.plan['number']}")
        entry["builder"]["changed"] = self.wt.head() != before
        return report, entry

    def _self_check_loop(self, cycle: int, entry: dict[str, Any], report: Any,
                         results: list[gates_mod.GateResult],
                         guard: list[Finding]) -> tuple[Any, list[gates_mod.GateResult],
                                                        list[Finding], list[Finding]]:
        """Self checks until one is clean: a fresh session of the builder's own model reads the
        change as a reviewer would; on findings the builder fixes them, the checks run and the
        self check goes again, up to `max_self_check_rounds`. Returns the last report, checks and
        path guard, and the findings still open (none after a clean self check). The report is
        None when a fix stopped to ask a person."""
        assert self.wt is not None
        cap = self.cfg.max_self_check_rounds
        rounds: list[dict[str, Any]] = []
        entry["self_check"] = rounds
        flagged: list[Finding] = []
        for n in range(1, cap + 1):
            self.check()
            verdict = self._review(cycle, report, results, [], role="self_check",
                                   context=SELF_CHECK_CONTEXT.format(n=n, cap=cap))
            flagged = list(guard) + list(verdict.blocking)
            if not verdict.readable:
                flagged.append(Finding("blocking", "self check",
                                       f"The self check gave no verdict: {verdict.why_unreadable}.",
                                       "the harness"))
            if not gates_mod.green(results):
                flagged.append(Finding("blocking", "checks", "Checks this change turned red "
                                       "must pass.", "the harness's gate run"))
            rounds.append({"n": n, "review": verdict.to_dict(), "flagged": len(flagged)})
            if not flagged or n == cap:
                break
            self.check()
            fixed, fix_entry = self._build_pass(
                cycle, "fix", self._fix_prompt(cycle, flagged, gates_mod.failures_text(results),
                                               label="Your self check's blocking findings"))
            rounds[-1]["fix"] = fix_entry["builder"]
            if fixed is None:
                return None, results, guard, flagged
            report = fixed
            guard = self._guard()
            self.check()
            results = self._checks()
            rounds[-1]["gates"] = [{**r.to_dict(), "tail": r.tail[-1500:]} for r in results]
        return report, results, guard, flagged

    def _item(self) -> None:
        conflicts = self._prepare()
        assert self.wt is not None
        self.carry_build, self.carry_review = self._cleared_notes(conflicts)
        if self.plan_seat is not None:
            self._planning()
            if self._rated_out():
                self._finish()
                return
        findings: list[Finding] = [_finding(f) for f in self.plan.get("previous_findings") or []]
        failures = ""
        report = verdicts.BuildReport("unknown", "", "", "")
        review: verdicts.Review | None = None
        results: list[gates_mod.GateResult] = []
        open_self_check: list[Finding] = []
        for cycle in range(1, self.cfg.max_review_cycles + 1):
            self.check()
            if cycle == 1:
                role, prompt = self._first_prompt(conflicts)
            else:
                role, prompt = "fix", self._fix_prompt(cycle, findings, failures)
            built, entry = self._build_pass(cycle, role, prompt)
            self.result["cycles"].append(entry)
            if built is None:
                return
            report = built
            guard = self._guard()
            if self._only_forbidden():
                return
            self.check()
            results = self._checks()
            entry["gates"] = [{**r.to_dict(), "tail": r.tail[-1500:]} for r in results]
            if self.self_check:
                checked, results, guard, open_self_check = self._self_check_loop(
                    cycle, entry, report, results, guard)
                if checked is None:
                    return
                report = checked
            if self.review_seat is None:
                # No seat here reviews in the run (a weak one only in a review run): a review
                # run judges it. But never with conflict markers left in it (#317 part 2): its
                # self checks may run out with them still open, as Devin's did on #203, #214
                # and #287, and nothing after this would catch them before CI.
                left, report = self._clear_markers(cycle, entry, report)
                if report is None:
                    return
                if left:
                    findings = [left]
                    self.result.update(status="not_approved", reason=f"conflict markers are "
                                       f"still in {left.where} after its last fix")
                    break
                self.result.update(status="built", reason=(
                    "built; no model on this subscription may review it, so it waits for a "
                    "review run" + (f", with {len(open_self_check)} self-check finding(s) still "
                                    "open" if open_self_check else "")))
                break
            self.check()
            context = ""
            if open_self_check:
                context = ((self._findings_text(findings) + "\n\n" if findings else "")
                           + "The builder's own self checks ran out with these findings still "
                           "open; judge them too:\n\n"
                           + self._findings_text(open_self_check, "Open self-check findings"))
            if self.carry_review:
                earlier = context or (self._findings_text(findings) if findings else "")
                context = (earlier + "\n\n" if earlier else "") + self.carry_review
            if self._catch_up():
                # `main` moved and merged cleanly: the checks and the review judge the merged head.
                results = self._checks()
                entry["gates"] = [{**r.to_dict(), "tail": r.tail[-1500:]} for r in results]
                entry["caught_up"] = self.base_sha
                guard = self._guard()
            review = self._review(cycle, report, results, findings, context=context)
            entry["review"] = review.to_dict()
            if not review.readable:
                self.result.update(status="not_approved", reason=review.why_unreadable)
                break
            if review.approved and gates_mod.green(results) and not guard:
                self.approved_sha = review.reviewed_sha
                self.result.update(status="approved", reason="the reviewer approved and every "
                                   "check passed")
                break
            findings = guard + review.blocking
            failures = gates_mod.failures_text(results)
            if not findings and failures:
                findings = [Finding("blocking", "checks", "Checks this change turned red must "
                                    "pass.", "the harness's gate run")]
        else:
            self.result.update(status="not_approved",
                               reason=f"no approval after {self.cfg.max_review_cycles} review cycles")
        self._last_checkpoint()
        status = self.result["status"]
        self.result.update(
            title=report.title or self.plan.get("title", ""),
            report=report.body,
            review=review.to_dict() if review else None,
            gates=[r.to_dict() for r in results],
            findings=[f.to_dict() for f in findings] if status not in ("approved", "built") else [],
            self_check_findings=[f.to_dict() for f in open_self_check],
        )
        self._finish()

    def _clear_markers(self, cycle: int, entry: dict[str, Any],
                       report: Any) -> tuple[Finding | None, Any]:
        """Before a change leaves the run with no reviewer in it: one fix pass when conflict
        markers are left in it and there is time. Returns the marker finding still open (None when
        there are none), and the builder's latest report (None when that pass stopped to ask a
        person, which ends the run)."""
        markers = self._unresolved_markers()
        if not markers:
            return None, report
        finding = Finding("blocking", ", ".join(markers[:8]),
                          f"Conflict markers are still in {', '.join(markers)}: resolve every "
                          "one, keeping both sides' meaning.", "git grep")
        if self.seconds_left() >= self.cfg.min_minutes_for_a_call * 60:
            fixed, fix = self._build_pass(cycle, "fix", self._fix_prompt(
                cycle, [finding], "", label="Conflict markers left in the change"))
            entry["marker_fix"] = fix["builder"]
            if fixed is None:
                return finding, None
            report = fixed
            markers = self._unresolved_markers()
            if not markers:
                return None, report
            finding = Finding("blocking", ", ".join(markers[:8]),
                              f"Conflict markers are still in {', '.join(markers)}.", "git grep")
        return finding, report

    def _catch_up(self) -> bool:
        """Before a review: merge `main` again when it moved since the run merged it and merges
        cleanly (#160, #317 part 12), so what is approved still merges; `main` moves about every
        half hour and a long run's approval was often stale before it was delivered. A merge
        that would conflict is left for the conflict path, and a conflict revision of a cleared
        change is left alone (deliver redoes its merge to check the carry). True when merged."""
        if self.carry_build or self.wt is None:
            return False
        default = self.cfg.default_branch
        self.repo.run("fetch", "--quiet", "--no-tags", "origin",
                      f"+refs/heads/{default}:refs/remotes/origin/{default}", check=False)
        now = self.repo.rev(self.base_ref)
        if not now or now == self.base_sha or self.wt.run(
                "merge-base", "--is-ancestor", now, "HEAD", check=False).returncode == 0:
            return False
        probe = self.wt.run("merge-tree", "--write-tree", "--no-messages", "HEAD", now,
                            check=False)
        if probe.returncode != 0:
            return False
        try:
            conflicts = self.wt.merge(now, self.who)
        except Exception:  # noqa: BLE001 - a merge that fails leaves the branch as it was
            self.wt.run("merge", "--abort", check=False)
            return False
        if conflicts:
            self.wt.run("merge", "--abort", check=False)
            return False
        self.base_sha = now
        self._base_wt = None  # the checks red on `main` are `main`'s new self's now
        self._base_gate_cache = {}
        return True

    def _only_forbidden(self) -> bool:
        """A build whose whole change was in paths the bot may not change (`bot/`, `.harness/`,
        `.github/`, ...): the path guard put it all back, so no fix or review can deliver it. The
        run ends now, asking a person, with nothing bundled, instead of spending its rounds."""
        assert self.wt is not None
        if self.plan["action"] != "build" or not self.put_back:
            return False
        if self.wt.changed_paths(self.base_ref):
            return False
        self.wt.run("reset", "--quiet", "--hard", self.start_sha)
        paths = ", ".join(f"`{p}`" for p in self.put_back[:8])
        self.result.update(
            status="blocked", reason="the change was all in paths the bot may not change",
            question=(f"Everything this run changed ({paths}) is in paths the bot may not "
                      "change, such as `bot/`, `.harness/` and `.github/`, so the harness put it "
                      "back and there is nothing to deliver. A person has to make this change, or "
                      "narrow the issue to the part outside those paths."))
        self._finish()
        return True

    def _last_checkpoint(self) -> None:
        """A halt or a stop said during the last review still counts: no finished change is
        handed on past one. The clock and the usage stop no longer matter here."""
        if self.probe is None:
            return
        found = self.probe(None)
        if found and found[1] in ("halt", "stop"):
            raise Interrupt(found[0], found[1])

    def _second_review(self) -> None:
        """A review run: a model reads a bot pull request (a weak one only for an easy item, where
        its approval counts toward the review rule). Nothing is built or pushed: the verdict goes
        to `deliver`, which merges once the approvals for the same commit meet the review rule
        (`review_rule.py`), or asks for a revision.
        Nothing is installed or run either: the reviewer holds another subscription's login, and
        the builder's code (a postinstall script, a test) must not run beside it. CI runs every
        check on the pull request before it can merge."""
        self._prepare(merge_main=False, install=False)
        assert self.wt is not None
        self.check()
        seat = self.seat_for("review")
        builder = str(self.plan.get("builder") or "an unknown model")
        approved = [str(f) for f in self.plan.get("approved") or []]
        so_far = (f"It already has an approval from {', '.join(f'`{f}`' for f in approved)}, "
                  "which is not evidence: judge it from scratch." if approved
                  else "Nobody has approved it yet.")
        context = (f"This is a review run. `{builder}` built this change; you are "
                   f"`{self.provider.family}` on `{seat.model}` ({seat.tier} tier). It merges once "
                   f"{review_rule.SUMMARY} approved the same commit. {so_far} Dependencies are "
                   "not installed in this run and you "
                   "should not run the branch's code: read it. CI runs every check on the pull "
                   "request before it can merge.")
        notes = str(self.plan.get("review_notes") or "").strip()
        if notes:
            context += ("\n\nA person asked for this review (`/harness review`) with these "
                        "notes:\n\n" + data(notes, "Their notes"))
        open_findings = [_finding(f) for f in self.plan.get("self_check_findings") or []]
        if open_findings:
            context += ("\n\nThe builder's own self checks ran out with these findings still "
                        "open; judge them too:\n\n"
                        + self._findings_text(open_findings, "Open self-check findings"))
        report = verdicts.BuildReport("done", str(self.plan.get("title") or ""), "",
                                      str(self.plan.get("pull") or ""))
        review = self._review(1, report, [], [], context=context, max_cycles=1)
        self.result["cycles"].append({"n": 1, "review": review.to_dict()})
        if not review.readable:
            self.result.update(status="failed", reason=review.why_unreadable)
            return
        self.result.update(
            status="reviewed",
            verdict="approve" if review.approved else "changes",
            reviewed_sha=review.reviewed_sha,
            review=review.to_dict(),
            findings=[f.to_dict() for f in review.blocking],
            reason=(f"`{self.provider.id}` ({seat.tier}) approved it" if review.approved
                    else f"`{self.provider.id}` ({seat.tier}) found {len(review.blocking)} "
                         "blocking problem(s)"),
        )

    # ------------------------------------------------------------------ endings

    def _save_wip(self, reason: str) -> None:
        """Commit what the worktree holds, so the deliver job can keep it."""
        if self.wt is None:
            return
        try:
            if self.wt.merge_in_progress() and self._unresolved_markers():
                self.wt.run("merge", "--abort", check=False)
            self._commit(f"bot: work in progress ({reason})")
        except Exception:  # noqa: BLE001 - a failed save leaves the branch as it was
            pass

    def _interrupted(self, stop: Interrupt) -> None:
        self._save_wip(stop.reason)
        self.result.update(status=KIND_STATUS.get(stop.kind, "interrupted"), reason=stop.reason,
                           interrupt=stop.kind, reset_at=stop.reset_at)
        self._finish()

    def _finish(self) -> None:
        """Record the branch's head and bundle what it has beyond where it started.

        An approved change is bundled exactly as the reviewer saw it: a commit that appeared
        after the review (a process the model left behind, say) is dropped."""
        if self.wt is None:
            return
        try:
            if self.result.get("status") == "approved" and self.approved_sha:
                if self.wt.head() != self.approved_sha:
                    self.result["dropped_after_review"] = self.wt.log(self.approved_sha)
                    self.wt.run("reset", "--quiet", "--hard", self.approved_sha)
            head = self.wt.head()
            self.result["head"] = head
            self.result["changed_paths"] = self.wt.changed_paths(self.base_ref)
            self.result["diffstat"] = self.wt.diffstat(self.base_ref)[-4000:]
            if head != self.start_sha:
                bundle = self.out_dir / "branch.bundle"
                exclude = [s for s in {self.base_sha, self.start_sha} if s]
                self.wt.bundle(bundle, str(self.plan["branch"]), exclude)
                self.result["bundle"] = bundle.name
        except Exception as exc:  # noqa: BLE001
            self.result["bundle_error"] = redact(str(exc))[:500]

    # ------------------------------------------------------------------ suggestions

    def _suggest(self) -> None:
        count = int(self.plan.get("count") or 0)
        if count <= 0:
            self.result.update(status="nothing", reason="the suggestion cap is full")
            return
        default = self.cfg.default_branch
        self.repo.run("fetch", "--quiet", "--no-tags", "origin",
                      f"+refs/heads/{default}:refs/remotes/origin/{default}")
        path = self.work_dir / "suggest"
        wt = worktree_add(self.repo, path, "bot-suggest", self.base_ref)
        self.check()
        prompt = self.render("suggest", repo=self.cfg.repo, count=count,
                             existing=self.plan.get("existing") or data("(none)", "Issues"))
        result = self.call("suggest", prompt, wt.cwd, reader=True)
        found = verdicts.suggestions(result.text, count)
        self.result.update(
            status="suggested",
            reason=f"{len(found)} suggestion(s)",
            suggestions=[{"title": s.title, "body": s.body} for s in found],
        )


def _finding(raw: Any) -> Finding:
    if isinstance(raw, Finding):
        return raw
    raw = raw if isinstance(raw, dict) else {}
    return Finding(str(raw.get("severity", "blocking")), str(raw.get("where", "?")),
                   str(raw.get("claim", "")), str(raw.get("evidence", "")))


def check_templates() -> list[str]:
    """Problems with the prompt files: a missing template or a stray `$`."""
    problems = []
    for name in prompts.NAMES:
        try:
            template = prompts.load(name)
        except FileNotFoundError:
            problems.append(f"prompts/{name}.md is missing")
            continue
        try:
            template.substitute({key: "x" for key in prompts.placeholders(name)})
        except (KeyError, ValueError) as exc:
            problems.append(f"prompts/{name}.md: {exc}")
    return problems


__all__ = ["Worker", "Interrupt", "check_templates"]
