"""The last job of a night run: check the model job's output and publish it.

This job holds the bot's token and runs no model. It does not trust the model job's files: the
item comes from the plan job's outputs and the branch is worked out again here, and the bundle
must hold the head the result names, descend from the commit the work started at, change no
forbidden path, and fast-forward the branch on GitHub. A pull request the reviewer approved gets
auto-merge, which merges it once the required CI checks pass.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from harness import asks
from harness import gates as gates_mod
from harness.clock import iso, parse_iso
from harness.config import (LABEL_BLOCKED, LABEL_BUILD, LABEL_NEEDS_REVIEW, LABEL_PR,
                            LABEL_PR_OPEN, LABEL_REVISE, LABEL_SUGGESTION)
from harness.context import Context
from harness.errors import GitError, GitHubError
from harness.git import Git, matches
from harness.plan import suggestions_due
from harness.queue import (branch_for_issue, candidates, label_names, open_pull_for_branch,
                           set_state_label)
from harness.state import item as state_item
from harness.state import record_usage, usage_refusal

REPORT_CHARS = 30_000
#: Runs that die without a result on one item before it is blocked.
DIED_LIMIT = 2
NO_RESULT = ("the model job left no result: it failed before the model started (the doctor "
             "step, the install or the CLI setup), or it was cancelled")


def load_result(out_dir: Path) -> dict[str, Any]:
    path = Path(out_dir) / "result.json"
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
        if isinstance(data, dict):
            return data
    except (OSError, ValueError):
        pass
    return {"status": "infra", "reason": NO_RESULT, "no_result": True}


class Deliverer:
    def __init__(self, ctx: Context, plan: dict[str, Any], out_dir: Path, repo_dir: Path, *,
                 action: str | None = None, number: int | None = None) -> None:
        self.ctx = ctx
        self.cfg = ctx.cfg
        self.gh = ctx.gh
        self.plan = dict(plan)
        if action:
            self.plan["action"] = action
        if number:
            self.plan["number"] = int(number)
        self.out_dir = Path(out_dir)
        self.repo = Git(repo_dir)
        self.result = load_result(out_dir)
        self.log: list[str] = []
        self.chain = True

    # ------------------------------------------------------------------ entry

    def run(self) -> dict[str, Any]:
        action = self.plan.get("action")
        usage, reset_at = self.result.get("usage"), self.result.get("reset_at")
        self.ctx.store.update(lambda s: record_usage(s, usage, reset_at, self.ctx.now()), "usage")
        if action == "suggest":
            self._suggestions()
        elif action in ("build", "revise"):
            self._rederive()
            self._item()
        else:
            return {"status": "nothing", "log": ["nothing was planned"]}
        self._chain()
        return {"status": self.result.get("status"), "log": self.log}

    def _rederive(self) -> None:
        """Work the branch out again from GitHub, whatever the plan file says."""
        number = int(self.plan["number"])
        if self.plan["action"] == "build":
            self.plan["branch"] = branch_for_issue(number)
            return
        pull = self.gh.get_pull(number)
        head = pull.get("head") or {}
        if (head.get("repo") or {}).get("full_name") != self.cfg.repo:
            raise GitHubError(f"#{number} is not a branch of {self.cfg.repo}", 0)
        self.plan["branch"] = head.get("ref", "")
        self.plan["bot_pr"] = LABEL_PR in label_names(pull)

    # ------------------------------------------------------------------ the branch

    def _open_pull(self) -> dict[str, Any] | None:
        branch = str(self.plan.get("branch") or "")
        return open_pull_for_branch(self.ctx, branch) if branch else None

    def _hold_auto_merge(self) -> None:
        """Turn auto-merge off on the branch's open PR before unapproved work lands on it."""
        pull = self._open_pull()
        if pull is not None and pull.get("auto_merge") and pull.get("node_id"):
            self._try(lambda: self.gh.disable_auto_merge(pull["node_id"]))
            self.log.append(f"auto-merge off on #{pull.get('number')}")

    def _publish(self, *, approved: bool) -> tuple[bool, str]:
        """Push the bundle's branch. `(pushed, problem)`; no bundle is `(False, "")`."""
        name = self.result.get("bundle")
        branch = str(self.plan.get("branch") or "")
        if not name or not branch:
            return False, ""
        bundle = self.out_dir / str(name)
        if not bundle.is_file():
            return False, "the result names a bundle the model job did not upload"
        default = self.cfg.default_branch
        self.repo.run("fetch", "--quiet", "--no-tags", "origin",
                      f"+refs/heads/{default}:refs/remotes/origin/{default}")
        self.repo.run("fetch", "--quiet", "--no-tags", "origin",
                      f"+refs/heads/{branch}:refs/remotes/origin/{branch}", check=False)
        local = f"deliver/{branch}"
        try:
            head = self.repo.fetch_bundle(bundle, branch, local)
        except GitError as exc:
            return False, f"the bundle could not be read: {exc}"
        if head != self.result.get("head"):
            return False, "the bundle's head is not the head the result names"
        start = str(self.result.get("start") or "")
        if not start or self.repo.run("merge-base", "--is-ancestor", start, head,
                                      check=False).returncode != 0:
            return False, "the bundle does not descend from the commit the work started at"
        remote = self.repo.rev(f"origin/{branch}")
        if remote and remote != start:
            return False, f"`{branch}` moved on GitHub while I worked, so I did not overwrite it"
        merged_main = self.repo.merge_base(f"origin/{default}", local)
        forbidden = self.repo.unsanctioned(local, [start, merged_main], self.cfg.forbidden_paths)
        if forbidden:
            return False, f"the change touches paths the bot may not change: {', '.join(forbidden)}"
        if not approved:
            self._hold_auto_merge()
        if self.cfg.dry_run:
            self.log.append(f"dry run: would push {head} to {branch}")
            return True, ""
        url = f"{self.cfg.server_url}/{self.cfg.repo}.git"
        try:
            self.repo.push(url, f"{local}:refs/heads/{branch}", self.cfg.write_token)
        except GitError as exc:
            return False, f"the push was refused: {exc}"
        self.log.append(f"pushed {head[:12]} to {branch}")
        return True, ""

    # ------------------------------------------------------------------ an item

    def _late_stop(self, number: int, status: str) -> str:
        """A stop or a halt that came after the model job's last checkpoint still counts."""
        if status in ("failed", "infra", "stopped"):
            return status
        state = self.ctx.store.load()
        record = state["items"].get(str(number), {})
        started = parse_iso(record.get("started_at"))
        stopped = parse_iso(record.get("stopped_at"))
        if record.get("stop_requested") and (started is None or stopped is None or stopped >= started):
            self.result.update(status="stopped", reason=f"stopped by @{record.get('stopped_by')}")
            return "stopped"
        if status in ("approved", "not_approved", "blocked") and (
                state.get("halted") or self.ctx.repo_halted()):
            self.result.update(status="interrupted", interrupt="halt",
                               reason="the bot was halted before this work could be published")
            return "interrupted"
        return status

    def _item(self) -> None:
        number = int(self.plan["number"])
        self._deliver_item(number)
        self._close_asks(number)

    def _deliver_item(self, number: int) -> None:
        status = self._late_stop(number, str(self.result.get("status")))
        kind = "revise" if self.plan["action"] == "revise" else "build"
        if status == "infra":
            self._infra(number, kind)
            return
        if status == "interrupted":
            self._interrupted(number, kind)
            return
        if kind == "revise":
            self._revise(number, status)
        else:
            self._build(number, status)
        self._settle(number)

    def _close_asks(self, number: int) -> None:
        """Settle the asks this run took (`asks`): 🎉 when it answered them, 😕 when it ended
        without an answer, or back to waiting when the item is queued again."""
        if not self._record(number).get("taken_asks"):
            return
        status = str(self.result.get("status"))
        blocked = LABEL_BLOCKED in self._labels(number)
        again = not blocked and status in ("interrupted", "infra", "failed")
        settled: list[str] = []

        def change(state: dict[str, Any]) -> None:
            entry = state_item(state, number)
            if again:
                asks.give_back(entry)
            else:
                settled[:] = asks.pop_taken(entry)

        self.ctx.store.update(change, f"asks #{number}")
        answered = not blocked and status == "approved"
        asks.react(self.gh, settled, asks.DONE if answered else asks.NO_ANSWER)

    def _labels(self, number: int) -> set[str]:
        return label_names(self.gh.get_issue(number))

    def _record(self, number: int) -> dict[str, Any]:
        return dict(self.ctx.store.load()["items"].get(str(number), {}))

    def _remember(self, number: int, **fields: Any) -> None:
        self.ctx.store.update(lambda s: state_item(s, number).update(fields), f"deliver #{number}")

    def _link(self) -> str:
        return f"[run]({self.cfg.run_url})" if self.cfg.run_url else "the run"

    def _requeue_label(self, number: int, kind: str) -> None:
        set_state_label(self.ctx, number, self._labels(number),
                        LABEL_REVISE if kind == "revise" else LABEL_BUILD)

    def _infra(self, number: int, kind: str) -> None:
        """A run that could not work. When the model job reported why (auth, the CLI, an install
        on untouched main), it is the environment's fault: nothing is charged and runs back off.
        When the job left no result at all (its runner died or timed out), it may be this item:
        it goes to the back of the queue, and two in a row block it."""
        self.chain = False
        reason = str(self.result.get("reason") or NO_RESULT)
        now = iso(self.ctx.now())
        if not self.result.get("no_result"):
            self._requeue_label(number, kind)
            self.ctx.store.update(lambda s: s.update(last_infra={"at": now, "reason": reason[:500]}),
                                  "infra")
            self.gh.create_comment(number, f"The run could not work on this ({self._link()}): "
                                   f"{reason}\n\nThat is not this item's fault; it stays queued "
                                   "for the next run.")
            return
        state = self.ctx.store.update(lambda s: state_item(s, number).update(
            died=int(state_item(s, number).get("died", 0)) + 1, queued_at=now), f"died #{number}")
        died = int(state_item(state, number).get("died", 0))
        if died >= DIED_LIMIT:
            set_state_label(self.ctx, number, self._labels(number), LABEL_BLOCKED)
            self._remember(number, forced=False)
            self.gh.create_comment(number, f"The run working on this died {died} times in a row "
                                   f"({self._link()}), so I stopped trying: {reason}. It may be "
                                   "too big or too heavy for one run; `/harness build` tries again.")
            return
        self._requeue_label(number, kind)
        self.gh.create_comment(number, f"The run working on this died ({self._link()}): {reason}. "
                               "It is back in the queue, behind the others.")

    def _interrupted(self, number: int, kind: str) -> None:
        interrupt = str(self.result.get("interrupt") or "budget")
        reason = self.result.get("reason")
        pushed, problem = (False, "")
        if kind == "build":
            pushed, problem = self._publish(approved=False)
        if interrupt == "budget":
            state = self.ctx.store.update(lambda s: state_item(s, number).update(
                interruptions=int(state_item(s, number).get("interruptions", 0)) + 1),
                f"interrupted #{number}")
            count = int(state_item(state, number).get("interruptions", 0))
            if count >= self.cfg.max_failures:
                set_state_label(self.ctx, number, self._labels(number), LABEL_BLOCKED)
                self._remember(number, forced=False)
                self.gh.create_comment(number, f"This ran out of time {count} runs in a row "
                                       f"({self._link()}), so I stopped. It is probably too big "
                                       "for one night; split it into smaller issues, or queue it "
                                       "again to keep going.")
                return
        self._requeue_label(number, kind)
        self._remember(number, last_findings=self.result.get("findings") or [])
        kept = " The work so far is on the branch." if pushed else ""
        self.gh.create_comment(number, f"Paused: {reason}.{kept} It stays queued and the next "
                               "run picks it up." + (f"\n\n{problem}" if problem else ""))

    def _fail(self, number: int, kind: str) -> None:
        """A run that produced nothing usable: requeue it, or block it after too many."""
        state = self.ctx.store.update(
            lambda s: state_item(s, number).update(
                failures=int(state_item(s, number).get("failures", 0)) + 1),
            f"failure #{number}")
        failures = int(state_item(state, number).get("failures", 0))
        reason = self.result.get("reason") or "unknown"
        if failures >= self.cfg.max_failures:
            set_state_label(self.ctx, number, self._labels(number), LABEL_BLOCKED)
            self._remember(number, forced=False)
            self.gh.create_comment(number, f"This failed {failures} times in a row "
                                   f"({self._link()}), so I stopped trying. Last error: {reason}"
                                   "\n\nFix the cause, then `/harness build` (or `revise`) "
                                   "queues it again.")
        else:
            self._requeue_label(number, kind)
            self.gh.create_comment(number, f"This run failed ({self._link()}): {reason}\n\nIt "
                                   f"is back in the queue (attempt {failures} of "
                                   f"{self.cfg.max_failures}).")

    def _settle(self, number: int) -> None:
        """After a finished run: clear the one-run flags, and go round again for a comment that
        arrived while the run held the thread (after a stop, one that came after it)."""
        status = str(self.result.get("status"))
        if status == "failed":
            return
        seen: dict[str, Any] = {}

        def change(state: dict[str, Any]) -> None:
            entry = state_item(state, number)
            seen.update(entry)
            entry.update(forced=False, interruptions=0, died=0, pending_request=False)

        self.ctx.store.update(change, f"settle #{number}")
        record = seen
        # `/harness stop` clears any earlier pending request, so one still set came after it.
        if not record.get("pending_request"):
            return
        target = number
        pr = int(record.get("pr") or 0) if status == "approved" else 0
        if self.plan["action"] == "build" and pr:
            target = pr
            # The asks that came in during the build are answered on the pull request now.
            self.ctx.store.update(lambda s: asks.add(state_item(s, pr),
                                                     asks.pop_waiting(state_item(s, number))),
                                  f"asks #{number} to #{pr}")
        is_pr = "pull_request" in self.gh.get_issue(target)
        if is_pr:
            pull = self.gh.get_pull(target)
            if pull.get("auto_merge") and pull.get("node_id"):
                self._try(lambda: self.gh.disable_auto_merge(pull["node_id"]))
        set_state_label(self.ctx, target, self._labels(target),
                        LABEL_REVISE if is_pr else LABEL_BUILD)
        self._remember(target, kind="revise" if is_pr else "build", queued_at=iso(self.ctx.now()),
                       stop_requested=False, source="request")
        self.gh.create_comment(target, "A comment arrived while I was working on this, so it is "
                               "queued again to answer it.")

    def _build(self, number: int, status: str) -> None:
        if status == "failed":
            self._fail(number, "build")
            return
        if status == "nothing":
            set_state_label(self.ctx, number, self._labels(number), None)
            return
        branch = str(self.plan["branch"])
        if status == "stopped":
            pushed, _ = self._publish(approved=False)
            set_state_label(self.ctx, number, self._labels(number), None)
            kept = f" What I had is on `{branch}`." if pushed else ""
            self.gh.create_comment(number, f"Stopped, as asked.{kept}")
            return
        approved = status == "approved"
        pushed, problem = self._publish(approved=approved)
        if problem.startswith("the push was refused"):
            self.result.update(status="infra", reason=problem, no_result=True)
            self._infra(number, "build")
            return
        if problem:
            set_state_label(self.ctx, number, self._labels(number), LABEL_BLOCKED)
            self.gh.create_comment(number, f"I could not publish this run's work ({self._link()}): "
                                   f"{problem}. It needs a person to look.")
            return
        started = self._record(number).get("started_at")
        if status == "blocked":
            set_state_label(self.ctx, number, self._labels(number), LABEL_BLOCKED)
            question = str(self.result.get("question") or self.result.get("reason") or "")
            kept = f" What I had is on `{branch}`." if pushed else ""
            self.gh.create_comment(number, f"I need a decision before I can build this.{kept}\n\n"
                                   f"{question}\n\nAnswer here, then `/harness build` (or "
                                   f"`@{self.cfg.bot_login} <your answer>`) queues it again.")
            self._remember(number, last_findings=[], question=question)
            return
        if not (self.repo.rev(f"origin/{branch}") or pushed):
            self._fail(number, "build")
            return
        body = self._pull_body(number, approved)
        title = str(self.result.get("title") or self.plan.get("title") or f"Build #{number}")
        pull = self._open_pull()
        if pull is None:
            pull = self._create_pull(title, branch, body, draft=not approved)
        else:
            self.gh.update_pull(int(pull["number"]), title=title, body=body)
            if approved and pull.get("draft"):
                self._try(lambda: self.gh.mark_ready(pull["node_id"]))
        pr = int(pull.get("number") or 0)
        if pr:
            self.gh.add_labels(pr, [LABEL_PR])
            self._remember(pr, issue=number, feedback_since=started, failures=0, kind="revise")
        issue_labels = self._labels(number)
        if approved:
            set_state_label(self.ctx, number, issue_labels, None)
            if LABEL_PR_OPEN not in issue_labels:
                self.gh.add_labels(number, [LABEL_PR_OPEN])
            asked = pr and LABEL_REVISE in self._labels(pr)
            if pr and not asked:
                set_state_label(self.ctx, pr, self._labels(pr), None)
            merge_note = ""
            if asked:
                merge_note = ("A revision was asked for on it while I built, so it stays queued "
                              "and auto-merge waits for it.")
            elif pr and self._record(number).get("pending_request"):
                merge_note = ("A comment arrived during the run, so auto-merge waits for the "
                              "revision that answers it.")
            elif pr:
                merge_note = self._auto_merge(self.gh.get_pull(pr))
            cycles = len(self.result.get("cycles") or [])
            self.gh.create_comment(number, f"Opened #{pr}. The adversarial reviewer approved it on "
                                   f"round {cycles} of {self.cfg.max_review_cycles}. {merge_note}")
            self._remember(number, last_findings=[], question="", failures=0, pr=pr)
        else:
            set_state_label(self.ctx, number, issue_labels, LABEL_BLOCKED)
            if pr and LABEL_REVISE not in self._labels(pr):
                set_state_label(self.ctx, pr, self._labels(pr), LABEL_BLOCKED)
            self.gh.create_comment(number, f"After {self.cfg.max_review_cycles} rounds the "
                                   f"reviewer still had blocking findings, so #{pr} stays a "
                                   f"draft and will not merge by itself ({self._link()}).\n\n"
                                   f"{self._findings_md()}\n\nComment `@{self.cfg.bot_login} "
                                   f"<guidance>` here or on #{pr} to have me try again with your "
                                   "notes.")
            self._remember(number, last_findings=self.result.get("findings") or [], pr=pr)

    def _create_pull(self, title: str, branch: str, body: str, *, draft: bool) -> dict[str, Any]:
        try:
            return self.gh.create_pull(title=title, head=branch, base=self.cfg.default_branch,
                                       body=body, draft=draft)
        except GitHubError as exc:
            existing = self._open_pull()
            if exc.status == 422 and existing is not None:
                self.gh.update_pull(int(existing["number"]), title=title, body=body)
                return existing
            raise

    def _revise(self, number: int, status: str) -> None:
        bot_pr = bool(self.plan.get("bot_pr"))
        if status == "failed":
            self._fail(number, "revise")
            return
        if status == "stopped":
            set_state_label(self.ctx, number, self._labels(number), None)
            self.gh.create_comment(number, "Stopped, as asked. I pushed nothing.")
            return
        if status == "blocked":
            set_state_label(self.ctx, number, self._labels(number), LABEL_BLOCKED)
            self._hold_auto_merge()
            question = self.result.get("question") or self.result.get("reason")
            self.gh.create_comment(number, f"I need a decision before I can revise this.\n\n"
                                   f"{question}")
            self._remember(number, question=str(question or ""))
            return
        started = self._record(number).get("started_at")
        if status != "approved":
            set_state_label(self.ctx, number, self._labels(number), LABEL_BLOCKED)
            self._hold_auto_merge()
            self.gh.create_comment(number, f"My revision did not pass review after "
                                   f"{self.cfg.max_review_cycles} rounds, so I pushed nothing "
                                   f"({self._link()}).\n\n{self._findings_md()}")
            self._remember(number, last_findings=self.result.get("findings") or [],
                           feedback_since=started)
            return
        pushed, problem = self._publish(approved=True)
        if problem:
            set_state_label(self.ctx, number, self._labels(number), LABEL_BLOCKED)
            self._hold_auto_merge()
            self.gh.create_comment(number, f"I could not push the revision ({self._link()}): "
                                   f"{problem}.")
            return
        set_state_label(self.ctx, number, self._labels(number), None)
        report = str(self.result.get("report") or "")[:REPORT_CHARS]
        rerun = not pushed and bool(self._record(number).get("ci_run_id"))
        self._remember(number, feedback_since=started, failures=0, source="", last_findings=[],
                       question="")
        note = ""
        if not pushed:
            note = "\n\nI changed nothing: the reviewer agreed no change was needed."
            if rerun:
                note += " I asked CI to run the failed jobs again."
                self._rerun_ci(number)
        merge_note = ""
        if bot_pr and not self._record(number).get("pending_request"):
            pull = self.gh.get_pull(number)
            if pull.get("draft"):
                self._try(lambda: self.gh.mark_ready(pull["node_id"]))
            merge_note = "\n\n" + self._auto_merge(pull)
        self.gh.create_comment(number, f"Revision {'pushed' if pushed else 'done'} "
                               f"({self._link()}); the adversarial reviewer approved it.{note}\n\n"
                               f"{report}{merge_note}")

    # ------------------------------------------------------------------ helpers

    def _try(self, action: Any) -> str:
        try:
            action()
            return ""
        except GitHubError as exc:
            return str(exc)

    def _changed(self, branch: str) -> list[str]:
        """What the branch changes against the default branch, as it now stands on GitHub."""
        default = self.cfg.default_branch
        self.repo.run("fetch", "--quiet", "--no-tags", "origin",
                      f"+refs/heads/{default}:refs/remotes/origin/{default}", check=False)
        local = f"deliver/{branch}"
        ref = local if self.repo.rev(local) else f"origin/{branch}"
        if not self.repo.rev(ref):
            self.repo.run("fetch", "--quiet", "--no-tags", "origin",
                          f"+refs/heads/{branch}:refs/remotes/origin/{branch}", check=False)
            if not self.repo.rev(ref):
                return []
        return self.repo.changed_paths(f"origin/{default}", ref)

    def _hand_to_a_person(self, pull: dict[str, Any], why: str) -> str:
        number = int(pull.get("number") or 0)
        if number:
            self.gh.add_labels(number, [LABEL_NEEDS_REVIEW])
            self._try(lambda: self.gh.request_reviewers(number, [self.cfg.operator]))
        return f"Auto-merge is off because {why}. @{self.cfg.operator}, it waits for you to merge it."

    def _auto_merge(self, pull: dict[str, Any]) -> str:
        """Turn on auto-merge only when nothing needs a person and CI must pass first."""
        if not self.cfg.auto_merge:
            return "Auto-merge is off in `.harness/config.json`; a person merges it."
        if not pull.get("node_id"):
            return ""
        branch = str((pull.get("head") or {}).get("ref") or self.plan.get("branch") or "")
        review = [p for p in self._changed(branch) if matches(p, self.cfg.review_paths)]
        if review:
            shown = ", ".join(f"`{p}`" for p in review[:8])
            return self._hand_to_a_person(pull, f"the change touches {shown}, which a person reviews")
        try:
            required = self.gh.required_checks(self.cfg.default_branch)
        except GitHubError as exc:
            return self._hand_to_a_person(pull, f"I could not read the branch protection ({exc})")
        missing = [c for c in self.cfg.required_checks if c not in (required or set())]
        if required is None or missing:
            what = "is not protected" if required is None else f"does not require {missing}"
            return self._hand_to_a_person(
                pull, f"`{self.cfg.default_branch}` {what}, so auto-merge would not wait for CI")
        error = self._try(lambda: self.gh.enable_auto_merge(pull["node_id"], self.cfg.merge_method))
        if not error:
            return "Auto-merge is on: it merges when every required check passes."
        if "clean status" in error.lower():
            # Every required check has already passed, so GitHub will not wait; merge it now.
            merged = self._try(lambda: self.gh.merge_pull(int(pull["number"]), self.cfg.merge_method))
            if not merged:
                return "Every required check had already passed, so I merged it."
        return self._hand_to_a_person(pull, f"GitHub refused it ({error})")

    def _rerun_ci(self, number: int) -> None:
        run_id = self._record(number).get("ci_run_id")
        if run_id:
            self._try(lambda: self.ctx.act.rerun_failed_jobs(run_id))

    def _findings_md(self) -> str:
        findings = self.result.get("findings") or []
        if not findings:
            return "_No findings were recorded._"
        lines = ["**Open findings:**"]
        for f in findings[:20]:
            lines.append(f"- `{f.get('where', '?')}`: {f.get('claim', '')}")
        return "\n".join(lines)

    def _pull_body(self, number: int, approved: bool) -> str:
        cycles = self.result.get("cycles") or []
        review = self.result.get("review") or {}
        report = str(self.result.get("report") or "_The builder wrote no description._")
        parts = [f"Closes #{number}", "", report[:REPORT_CHARS], ""]
        if approved:
            parts.append(f"### Adversarial review\n\nApproved on round {len(cycles)} of "
                         f"{self.cfg.max_review_cycles}.")
            notes = [f for f in review.get("findings") or [] if f.get("severity") == "note"]
            if notes:
                parts.append("\nNotes the reviewer left (not blocking):\n")
                parts += [f"- `{f.get('where')}`: {f.get('claim')}" for f in notes[:20]]
        else:
            parts.append("### Not approved\n\nThe reviewer still had blocking findings after "
                         f"{len(cycles)} rounds, so this is a draft and will not merge by itself.")
            parts.append("")
            parts.append(self._findings_md())
        rows = ["| Round | Builder | Checks | Review |", "|---|---|---|---|"]
        for c in cycles:
            builder = c.get("builder") or {}
            checks = c.get("gates") or []
            red = [g["name"] for g in checks if not g.get("ok") and not g.get("pre_existing")]
            verdict = (c.get("review") or {}).get("verdict", "-")
            blocking = len([f for f in (c.get("review") or {}).get("findings") or []
                            if f.get("severity") == "blocking"])
            rows.append(f"| {c.get('n')} | {builder.get('role')}, {builder.get('minutes')} min | "
                        f"{'red: ' + ', '.join(red) if red else 'green'} | {verdict}"
                        f"{f' ({blocking} blocking)' if blocking else ''} |")
        results = [gates_mod.GateResult(**{k: g.get(k) for k in (
            "name", "run", "ok", "exit_code", "seconds", "tail", "pre_existing", "skipped")})
            for g in self.result.get("gates") or []]
        parts += ["", "<details><summary>Rounds and checks</summary>", "", *rows, "",
                  gates_mod.table(results), "", "</details>", ""]
        run = f" ([run]({self.cfg.run_url}))" if self.cfg.run_url else ""
        parts.append(f"---\nBuilt overnight by @{self.cfg.bot_login}{run}. Comment "
                     f"`@{self.cfg.bot_login} <what to change>` or `/harness revise <notes>` to ask "
                     "for a revision, or `/harness stop` to stop it.")
        return "\n".join(parts)

    # ------------------------------------------------------------------ suggestions

    def _suggestions(self) -> None:
        if self.result.get("status") == "infra":
            self.chain = False
        if self.result.get("status") != "suggested":
            # A survey that did not finish gives its slot back: it is due again as before.
            before = {"last_run": self.plan.get("previous_last_run"),
                      "requested": bool(self.plan.get("was_requested"))}

            def unfinished(s: dict[str, Any]) -> None:
                s["suggest"].update(before)
                asks.give_back(s["suggest"])

            self.ctx.store.update(unfinished, "survey unfinished")
        else:
            settled: list[str] = []

            def done(s: dict[str, Any]) -> None:
                settled[:] = asks.pop_taken(s["suggest"])

            self.ctx.store.update(done, "survey asks")
            asks.react(self.gh, settled, asks.DONE)
        found = self.result.get("suggestions") or []
        open_now = [i for i in self.gh.list_issues(labels=LABEL_SUGGESTION) if "pull_request" not in i]
        room = max(0, self.cfg.suggestions_max_open - len(open_now))
        made = []
        for entry in found[:room]:
            title = str(entry.get("title") or "").strip()
            body = str(entry.get("body") or "").strip()
            if not title or not body:
                continue
            footer = (f"\n\n---\nSuggested by @{self.cfg.bot_login} while the queue was empty. "
                      "Add the `bot:build` label (or comment `/harness build`) to have it built; "
                      "close it to say no.")
            issue = self.gh.create_issue(title, body + footer, [LABEL_SUGGESTION])
            made.append(issue.get("number"))
        self.log.append(f"opened suggestions: {made}")

    # ------------------------------------------------------------------ the next run

    def _chain(self) -> None:
        """Start the next run at once when there is more to do inside the window."""
        ctx = self.ctx
        now = ctx.now()
        if not self.chain or not ctx.window.is_open(now) or ctx.repo_halted():
            return
        state = ctx.store.load()
        if state.get("halted") or usage_refusal(state, dict(self.cfg.usage_stop), now):
            return
        if candidates(ctx, state) or suggestions_due(ctx, state):
            ctx.dispatch()
            self.log.append("dispatched the next run")
