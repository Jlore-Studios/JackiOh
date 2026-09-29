"""The model job: one item through builder -> checks -> adversarial review, or a suggestion survey.

This job holds no GitHub write credential. It writes `result.json`, a git bundle of the branch and
the redacted transcripts to the output directory; the deliver job checks the bundle itself and
pushes it. Prompts are read into memory when the job starts, so nothing the model writes to disk
can change what a later call is asked.
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
from harness.git import Git, Identity, is_forbidden, worktree_add
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
SETTINGS_FILES = (".claude/settings.json", ".claude/settings.local.json", ".mcp.json")


class Interrupt(Exception):
    """Stop the item now and hand what exists to the deliver job."""

    def __init__(self, reason: str, reset_at: str | None = None, stopped: bool = False) -> None:
        super().__init__(reason)
        self.reason = reason
        self.reset_at = reset_at
        self.stopped = stopped


#: What a probe returns when the run must stop: the reason, and whether a person stopped this
#: item (as opposed to a halt, a usage stop or the clock, after which the item is requeued).
Probe = Callable[[dict | None], "tuple[str, bool] | None"]


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
        self.now = now or (lambda: clock_now(cfg.now_override))
        self.env = dict(env) if env is not None else child_env()
        self.started = self.now()
        self.deadline = self.started + timedelta(minutes=cfg.job_budget_minutes)
        self.templates = {name: prompts.load(name) for name in prompts.NAMES}
        self.system = self.templates["system"].substitute(bot=cfg.bot_login, repo=cfg.repo)
        self.who = Identity.bot(cfg.bot_login, cfg.bot_user_id)
        self.last_usage: dict | None = None
        self.calls = 0
        self.wt: Git | None = None
        self.base_ref = f"origin/{cfg.default_branch}"
        self.base_sha = ""
        self.start_sha = ""
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
        }

    # ------------------------------------------------------------------ plumbing

    def seconds_left(self) -> float:
        return (self.deadline - self.now()).total_seconds()

    def check(self) -> None:
        """Raise Interrupt when a halt, a stop, the usage stop or the clock says to stop."""
        if self.seconds_left() < self.cfg.min_minutes_for_a_call * 60:
            raise Interrupt("the run's time budget is spent")
        if self.probe is not None:
            found = self.probe(self.last_usage)
            if found:
                reason, stopped = found
                raise Interrupt(reason, stopped=stopped)

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
        request = RunRequest(
            role=role,
            prompt=prompt,
            cwd=cwd,
            system_append=self.system,
            allowed_tools=READER_TOOLS if reader else BUILDER_TOOLS,
            disallowed_tools=READER_DENY if reader else BUILDER_DENY,
            max_turns=int(self.cfg.max_turns[role]),
            timeout_s=max(60, timeout),
            model=self.cfg.model,
            effort=self.cfg.effort,
            transcript=transcript,
        )
        result = self.runner.run(request)
        if result.usage:
            self.last_usage = result.usage
        if result.rate_limited:
            raise Interrupt("the subscription's usage limit was reached", result.reset_at)
        return result

    def write_result(self) -> Path:
        self.out_dir.mkdir(parents=True, exist_ok=True)
        self.result["usage"] = self.last_usage
        self.result["finished_at"] = iso(self.now())
        self.result["model_calls"] = self.calls
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

    def _prepare(self) -> list[str]:
        number = int(self.plan["number"])
        branch = str(self.plan["branch"])
        default = self.cfg.default_branch
        self.repo.run("fetch", "--quiet", "--no-tags", "origin",
                      f"+refs/heads/{default}:refs/remotes/origin/{default}")
        self.repo.run("fetch", "--quiet", "--no-tags", "origin",
                      f"+refs/heads/{branch}:refs/remotes/origin/{branch}", check=False)
        remote = f"origin/{branch}"
        has_remote = self.repo.rev(remote) is not None
        if self.plan["action"] == "revise" and not has_remote:
            raise RuntimeError(f"the pull request's branch {branch} is not on origin")
        start = remote if has_remote else self.base_ref
        self.base_sha = self.repo.rev(self.base_ref) or ""
        self.start_sha = self.repo.rev(start) or ""
        self.wt = worktree_add(self.repo, self.work_dir / f"item-{number}", branch, start)
        self.result.update(branch=branch, base=self.base_sha, start=self.start_sha,
                           remote_branch_existed=has_remote)
        conflicts: list[str] = []
        if has_remote:
            conflicts = self.wt.merge(self.base_ref, self.who)
        install = gates_mod.run_gate(self.cfg.install, self.wt.cwd, self.env, int(self.seconds_left()))
        if not install.ok:
            raise RuntimeError(f"dependency install failed (exit {install.exit_code}):\n{install.tail[-3000:]}")
        return conflicts

    def _branch_state(self) -> str:
        assert self.wt is not None
        log = self.wt.log(self.base_ref)
        if not log:
            return "The branch starts at `main` with no commits of its own yet."
        stat = self.wt.diffstat(self.base_ref)
        return data(f"{log}\n\n{stat}", "Commits on this branch beyond main")

    def _gate_list(self) -> str:
        lines = ["The harness's checks, run in this order after you stop:"]
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
                number=number, title=plan.get("title", ""), repo=self.cfg.repo,
                branch=plan["branch"], base=self.base_sha, source=plan.get("source", "request"),
                pull=plan.get("pull", ""), issue=plan.get("issue", ""),
                feedback=plan.get("feedback", ""), conflicts=conflict_text,
                branch_state=self._branch_state(), gate_list=self._gate_list(),
            )
            return "revise", prompt
        previous = ""
        prior = plan.get("previous_findings") or []
        if prior:
            previous = ("An earlier run left this branch unfinished. The last review's blocking "
                        "findings were:\n\n" + self._findings_text([_finding(f) for f in prior]))
        if conflicts:
            previous += ("\n\n`main` moved since then; merging it left conflict markers in: "
                         + ", ".join(f"`{c}`" for c in conflicts)
                         + ". Resolve them, keeping both sides' meaning. Do not commit.")
        prompt = self.render(
            "build",
            number=number, title=plan.get("title", ""), repo=self.cfg.repo,
            branch=plan["branch"], base=self.base_sha, thread=plan.get("thread", ""),
            branch_state=self._branch_state(), previous=previous, gate_list=self._gate_list(),
        )
        return "build", prompt

    def _fix_prompt(self, cycle: int, findings: list[Finding], failures: str) -> str:
        return self.render(
            "fix",
            number=self.plan["number"], title=self.plan.get("title", ""), repo=self.cfg.repo,
            branch=self.plan["branch"], base=self.base_sha, cycle=cycle,
            max_cycles=self.cfg.max_review_cycles, thread=self.plan.get("thread", ""),
            branch_state=self._branch_state(), findings=self._findings_text(findings),
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

    def _guard(self) -> list[Finding]:
        """Put back forbidden paths and report them, plus conflict markers and an empty change."""
        assert self.wt is not None
        found: list[Finding] = []
        changed = self.wt.changed_paths(self.base_ref)
        touched = [p for p in changed if is_forbidden(p, self.cfg.forbidden_paths)]
        if touched:
            base = self.wt.merge_base(self.base_ref)
            self.wt.restore_paths(base, touched)
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

    def _mark_pre_existing(self, results: list[gates_mod.GateResult]) -> None:
        for result in results:
            if result.ok or result.skipped:
                continue
            if result.name not in self._base_gate_cache:
                base = self._base_worktree()
                if base is None:
                    return
                again = gates_mod.run_gate(
                    next(g for g in self.cfg.gates if g.name == result.name),
                    base.cwd, self.env, int(self.seconds_left()),
                )
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
                results: list[gates_mod.GateResult], previous: list[Finding]) -> verdicts.Review:
        assert self.wt is not None
        diff, cut = self.wt.diff(self.base_ref, max_chars=DIFF_IN_PROMPT)
        note = ("The diff below is cut short; run `git diff "
                f"{self.base_sha}...HEAD` for the rest." if cut else
                "The whole diff against the base is below.")
        prompt = self.render(
            "review",
            number=self.plan["number"], title=self.plan.get("title", ""), repo=self.cfg.repo,
            cycle=cycle, max_cycles=self.cfg.max_review_cycles, branch=self.plan["branch"],
            base=self.base_sha, thread=self.plan.get("thread", ""),
            report=data(report.body or "(the builder wrote no report)", "Builder's report"),
            gates=gates_mod.table(results),
            previous_findings=(self._findings_text(previous) if previous
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
            results = gates_mod.run_all(self.cfg.gates, self.wt.cwd, self.env, self.seconds_left)
            self._mark_pre_existing(results)
            entry["gates"] = [{**r.to_dict(), "tail": r.tail[-1500:]} for r in results]
            self.check()
            review = self._review(cycle, report, results, findings)
            entry["review"] = review.to_dict()
            if not review.readable:
                self.result.update(status="not_approved", reason="the reviewer's answer could "
                                   "not be read twice in a row")
                break
            if review.approved and gates_mod.green(results) and not guard:
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
        self.result.update(
            title=report.title or self.plan.get("title", ""),
            report=report.body,
            review=review.to_dict() if review else None,
            gates=[r.to_dict() for r in results],
            findings=[f.to_dict() for f in findings] if self.result["status"] != "approved" else [],
        )
        self._finish()

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
        self.result.update(status="stopped" if stop.stopped else "interrupted",
                           reason=stop.reason, reset_at=stop.reset_at)
        self._finish()

    def _finish(self) -> None:
        """Record the branch's head and bundle what it has beyond where it started."""
        if self.wt is None:
            return
        try:
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
