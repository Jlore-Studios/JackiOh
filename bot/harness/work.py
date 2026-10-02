"""The model job: one item through builder -> checks -> adversarial review, a second model's
review of a bot pull request, or a suggestion survey, all on the one subscription `plan` chose.

This job holds no GitHub write credential. It writes `result.json` and a git bundle of the branch
to the output directory; the deliver job checks the bundle itself and pushes it. Prompts are read
into memory when the job starts, so nothing the model writes to disk can change what a later
call is asked.
"""

from __future__ import annotations

import json
from datetime import datetime, timedelta
from pathlib import Path
from typing import Any, Callable

from harness import gates as gates_mod
from harness import prompts, verdicts
from harness.clock import iso, now as clock_now
from harness.config import Config, child_env
from harness.git import Git, Identity, worktree_add
from harness.prompts import data
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
SETTINGS_FILES = (".claude/settings.json", ".claude/settings.local.json", ".mcp.json")
MANIFESTS = ("package.json", "pnpm-lock.yaml", "pnpm-workspace.yaml", ".npmrc")

#: How an interruption ends the item: `stop` and `infra` have statuses of their own; the rest
#: (`budget`, `usage`, `halt`) are `interrupted` and requeue.
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
        self.minutes = 0.0
        self.build_transcript: Path | None = None
        self.who = Identity.bot(cfg.bot_login, cfg.bot_user_id)
        self.last_usage: dict | None = None
        self.calls = 0
        self.wt: Git | None = None
        self.base_ref = f"origin/{cfg.default_branch}"
        self.base_sha = ""
        self.start_sha = ""
        self.installed_at: str | None = None
        self.approved_sha: str | None = None
        self._base_wt: Git | None = None
        self._base_gate_cache: dict[str, bool] = {}
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
        }

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
        request = RunRequest(
            role=role,
            prompt=prompt,
            cwd=cwd,
            system_append=self.system,
            allowed_tools=READER_TOOLS if reader else BUILDER_TOOLS,
            disallowed_tools=READER_DENY if reader else BUILDER_DENY,
            max_turns=int(self.cfg.max_turns[role]),
            timeout_s=max(60, timeout),
            model=self.provider.model,
            effort=self.provider.effort,
            transcript=transcript,
            read_only=reader,
            extra_dirs=self._git_dirs(cwd),
        )
        result = self.runner.run(request)
        if self.after_call is not None:
            self.after_call()
        self.minutes += result.duration_s / 60
        if result.usage:
            self.last_usage = result.usage
        if result.rate_limited:
            raise Interrupt(f"`{self.provider.id}` reached its usage limit", "usage",
                            result.reset_at)
        if result.infra:
            raise Interrupt(f"the {self.provider.cli} CLI could not run: {result.error}", "infra")
        return result

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
            action = self.plan.get("action")
            if action == "suggest":
                self._suggest()
            elif action in ("build", "revise"):
                self._item()
            elif action == "review":
                self._second_review()
            else:
                self.result.update(status="nothing", reason="the plan had nothing to do")
        except Interrupt as stop:
            self._interrupted(stop)
        except Exception as exc:  # noqa: BLE001 - every failure must still leave a result
            self.result.update(status="failed", reason=redact(f"{type(exc).__name__}: {exc}")[:2000])
            self._save_wip("the harness failed")
        finally:
            self.write_result()
        return self.result

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
        self.wt = worktree_add(self.repo, self.work_dir / f"item-{number}", branch, start)
        self._exclude_notes()
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
            raise Interrupt(f"dependency install failed on untouched main (exit "
                            f"{installed.exit_code}): {installed.tail[-1500:]}", "infra")
        return conflicts

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

    def _handoff(self) -> dict[str, Any] | None:
        """What the next agent needs if this run did not finish the item: the builder's notes
        and the end of its last session. None when there is neither."""
        if self.wt is None:
            return None
        notes_path = self.wt.cwd / NOTES_FILE
        notes = ""
        if notes_path.is_file():
            notes = notes_path.read_text(encoding="utf-8", errors="replace")[-NOTES_CHARS:]
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
        parts = [f"## Picking up from another agent\n\nAn earlier run on "
                 f"`{handoff.get('provider', '?')}` ({handoff.get('family', '?')}) worked on this "
                 f"and stopped: {handoff.get('reason') or 'no reason recorded'}. Its work so far is "
                 "on the branch. Below are the notes it kept and the end of its session. Check "
                 "them against the diff and re-run the checks before you trust any of it."]
        if str(handoff.get("notes") or "").strip():
            parts.append(data(str(handoff["notes"]), "Its notes"))
        if str(handoff.get("trail") or "").strip():
            parts.append(data(str(handoff["trail"]), "The end of its session"))
        return "\n\n" + "\n\n".join(parts)

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
        lines += [f"- {g.name}: `{g.run}`" for g in self.cfg.gates]
        lines.append("CI on the pull request also runs the fuzz gate, coverage, the AI gates, "
                     "the Postgres suites and the Cypress e2e specs before anything merges.")
        return "\n".join(lines)

    def _findings_text(self, findings: list[Finding]) -> str:
        if not findings:
            return "No blocking findings were recorded."
        body = "\n".join(f.markdown() for f in findings)
        return data(body, "Blocking findings")

    def _first_prompt(self, conflicts: list[str]) -> tuple[str, str]:
        plan = self.plan
        number = plan["number"]
        if plan["action"] == "revise":
            conflict_text = (
                data("\n".join(conflicts), "Files merged with conflict markers")
                if conflicts else "The merge of `main` into the branch was clean."
            )
            prompt = self.render(
                "revise",
                number=number, repo=self.cfg.repo, branch=plan["branch"], base=self.base_sha,
                source=plan.get("source", "request"), pull=plan.get("pull", ""),
                issue=plan.get("issue", ""), feedback=plan.get("feedback", ""),
                conflicts=conflict_text, branch_state=self._branch_state(),
                gate_list=self._gate_list(),
            )
            return "revise", prompt + self._handoff_text()
        previous = ""
        prior = plan.get("previous_findings") or []
        if prior:
            previous = ("An earlier run left this branch unfinished. The last review's blocking "
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
        prompt = self.render(
            "build",
            number=number, repo=self.cfg.repo, branch=plan["branch"], base=self.base_sha,
            thread=plan.get("thread", ""), branch_state=self._branch_state(), previous=previous,
            gate_list=self._gate_list(),
        )
        return "build", prompt + self._handoff_text()

    def _fix_prompt(self, cycle: int, findings: list[Finding], failures: str) -> str:
        return self.render(
            "fix",
            number=self.plan["number"], repo=self.cfg.repo, branch=self.plan["branch"],
            base=self.base_sha, cycle=cycle, max_cycles=self.cfg.max_review_cycles,
            thread=self.plan.get("thread", ""), branch_state=self._branch_state(),
            findings=self._findings_text(findings),
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
        proc = self.wt.run("grep", "-l", "-E", r"^(<<<<<<< |>>>>>>> )", "--", *changed, check=False)
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
                            for g in self.cfg.gates]
                return results
        results += gates_mod.run_all(self.cfg.gates, self.wt.cwd, self.env, self.seconds_left)
        self._mark_pre_existing(results)
        return results

    def _mark_pre_existing(self, results: list[gates_mod.GateResult]) -> None:
        for result in results:
            if result.ok or result.skipped or result.name == self.cfg.install.name:
                continue
            gate = next((g for g in self.cfg.gates if g.name == result.name), None)
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
                context: str = "", max_cycles: int | None = None) -> verdicts.Review:
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
            result = self.call("review", prompt, self.wt.cwd, reader=True)
            if self.wt.head() != head:
                self.wt.run("reset", "--quiet", "--hard", head)
            if self.wt.dirty():
                self.wt.discard_worktree_changes()
            review = verdicts.review(result.text)
            if review.readable:
                break
            self.check()
        review.reviewed_sha = head
        return review

    def _item(self) -> None:
        conflicts = self._prepare()
        assert self.wt is not None
        findings: list[Finding] = [_finding(f) for f in self.plan.get("previous_findings") or []]
        failures = ""
        report = verdicts.BuildReport("unknown", "", "", "")
        review: verdicts.Review | None = None
        results: list[gates_mod.GateResult] = []
        for cycle in range(1, self.cfg.max_review_cycles + 1):
            self.check()
            if cycle == 1:
                role, prompt = self._first_prompt(conflicts)
            else:
                role, prompt = "fix", self._fix_prompt(cycle, findings, failures)
            built = self.call(role, prompt, self.wt.cwd, reader=False)
            report = verdicts.build_report(built.text)
            entry: dict[str, Any] = {
                "n": cycle,
                "builder": {"role": role, "ok": built.ok, "turns": built.turns,
                            "minutes": round(built.duration_s / 60, 1), "status": report.status,
                            "error": built.error, "timed_out": built.timed_out},
            }
            self.result["cycles"].append(entry)
            if report.status == "blocked":
                self._save_wip("the builder needs a decision")
                self.result.update(status="blocked", question=report.question, report=report.body,
                                   reason="the builder needs a person to decide something")
                self._finish()
                return
            self._commit(f"bot: {role} pass {cycle} for #{self.plan['number']}")
            guard = self._guard()
            self.check()
            results = self._checks()
            entry["gates"] = [{**r.to_dict(), "tail": r.tail[-1500:]} for r in results]
            self.check()
            review = self._review(cycle, report, results, findings)
            entry["review"] = review.to_dict()
            if not review.readable:
                self.result.update(status="not_approved", reason="the reviewer's answer could "
                                   "not be read twice in a row")
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
        self.result.update(
            title=report.title or self.plan.get("title", ""),
            report=report.body,
            review=review.to_dict() if review else None,
            gates=[r.to_dict() for r in results],
            findings=[f.to_dict() for f in findings] if self.result["status"] != "approved" else [],
        )
        self._finish()

    def _last_checkpoint(self) -> None:
        """A halt or a stop said during the last review still counts: no finished change is
        handed on past one. The clock and the usage stop no longer matter here."""
        if self.probe is None:
            return
        found = self.probe(None)
        if found and found[1] in ("halt", "stop"):
            raise Interrupt(found[0], found[1])

    def _second_review(self) -> None:
        """A model of another family reads a bot pull request its builder's model approved.
        Nothing is built or pushed: the verdict goes to `deliver`, which merges or asks for a
        revision. Nothing is installed or run either: the reviewer holds another subscription's
        login, and the builder's code (a postinstall script, a test) must not run beside it. CI
        runs every check on the pull request before it can merge."""
        self._prepare(merge_main=False, install=False)
        assert self.wt is not None
        self.check()
        builder = str(self.plan.get("builder") or "an unknown model")
        context = (f"This is a second review. `{builder}` built this change and its own reviewer "
                   f"approved it; you are `{self.provider.family}`, a different model, and the "
                   "change merges only if you approve it too. Judge it from scratch: the first "
                   "approval is not evidence. Dependencies are not installed in this run and you "
                   "should not run the branch's code: read it. CI runs every check on the pull "
                   "request before it can merge.")
        report = verdicts.BuildReport("done", str(self.plan.get("title") or ""), "",
                                      str(self.plan.get("pull") or ""))
        review = self._review(1, report, [], [], context=context, max_cycles=1)
        self.result["cycles"].append({"n": 1, "review": review.to_dict()})
        if not review.readable:
            self.result.update(status="failed", reason="the second reviewer's answer could not "
                               "be read twice in a row")
            return
        self.result.update(
            status="reviewed",
            verdict="approve" if review.approved else "changes",
            reviewed_sha=review.reviewed_sha,
            review=review.to_dict(),
            findings=[f.to_dict() for f in review.blocking],
            reason=(f"`{self.provider.id}` approved it" if review.approved
                    else f"`{self.provider.id}` found {len(review.blocking)} blocking problem(s)"),
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
