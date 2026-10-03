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
from harness import plan as plan_mod
from harness import providers as providers_mod
from harness import vault
from harness.clock import iso, parse_iso
from harness.config import (LABEL_BLOCKED, LABEL_BUILD, LABEL_CROSS, LABEL_NEEDS_REVIEW,
                            LABEL_PR, LABEL_PR_OPEN, LABEL_REVISE, LABEL_SHITTER,
                            LABEL_SUGGESTION, STATE_BRANCH)
from harness.context import Context
from harness.errors import GitError, GitHubError
from harness.git import Git, matches
from harness.queue import (PRIORITY_TIERS, branch_for_issue, label_names, open_pull_for_branch,
                           set_state_label)
from harness.state import item as state_item
from harness.redact import redact

REPORT_CHARS = 30_000
#: How much of a stopped run's notes and session trail is kept for the next agent.
HANDOFF_NOTES = 8000
HANDOFF_TRAIL = 6000
#: Runs that die without a result on one item before it is blocked.
DIED_LIMIT = 2
#: Writes of a refreshed login tried before giving up (another writer raced it).
VAULT_ATTEMPTS = 4
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
                 action: str | None = None, number: int | None = None,
                 provider: str | None = None) -> None:
        self.ctx = ctx
        self.cfg = ctx.cfg
        self.gh = ctx.gh
        self.plan = dict(plan)
        if action:
            self.plan["action"] = action
        if number:
            self.plan["number"] = int(number)
        if provider:
            self.plan["provider"] = provider
        pool = ctx.cfg.pool
        self.provider = pool.get(self.plan.get("provider")) or pool.get(
            providers_mod.LEGACY_PROVIDER) or pool.ordered()[0]
        self.out_dir = Path(out_dir)
        self.repo = Git(repo_dir)
        self.result = load_result(out_dir)
        self.log: list[str] = []
        self.chain = True

    # ------------------------------------------------------------------ entry

    def run(self) -> dict[str, Any]:
        action = self.plan.get("action")
        usage, reset_at = self.result.get("usage"), self.result.get("reset_at")
        # The refreshed login first: a rotated one that is lost cannot be recovered.
        self._store_vault()
        try:
            minutes = float(self.result.get("minutes") or 0)
        except (TypeError, ValueError):
            minutes = 0.0
        self.ctx.store.update(lambda s: providers_mod.note_usage(
            s, self.provider.id, usage, reset_at, self.ctx.now(), minutes),
            f"usage {self.provider.id}")
        if action == "suggest":
            self._suggestions()
        elif action in ("build", "revise", "review"):
            self._rederive()
            self._item()
        else:
            return {"status": "nothing", "log": ["nothing was planned"]}
        self._chain()
        return {"status": self.result.get("status"), "log": self.log}

    def _store_vault(self) -> None:
        """Keep a login the CLI refreshed during the run (`vault.py`) for this provider's next
        run. The model job sealed it; this job cannot open it and only checks its shape."""
        path = self.out_dir / "vault.enc"
        if not path.is_file():
            return
        text = path.read_text(encoding="utf-8", errors="replace").strip()
        if not vault.looks_sealed(text):
            self.log.append("ignored a vault that is not sealed")
            return
        where = plan_mod.vault_path(self.provider.id)
        problem = ""
        for _ in range(VAULT_ATTEMPTS):
            try:
                _, sha = self.gh.get_file(where, STATE_BRANCH)
                self.gh.put_file(where, text + "\n", branch=STATE_BRANCH, sha=sha,
                                 message=f"vault: {self.provider.id} [skip ci]")
                self.log.append(f"kept the refreshed login of {self.provider.id}")
                return
            except GitHubError as exc:
                problem = str(exc)
                if exc.status not in (409, 422):
                    break  # only a race with another writer is worth trying again
        self.log.append(f"could not keep the refreshed login of {self.provider.id}: {problem}")

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

    def _hold_auto_merge(self, number: int | None = None) -> None:
        """Turn auto-merge off on the branch's open PR (or on PR `number`) before unapproved
        work lands on it."""
        pull = self.gh.get_pull(number) if number else self._open_pull()
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
        self._keep_handoff(number)
        self._close_asks(number)

    def _deliver_item(self, number: int) -> None:
        thread = self.gh.get_issue(number)
        if thread.get("state") != "open":
            self._closed(number, thread)
            return
        status = self._late_stop(number, str(self.result.get("status")))
        if self.plan["action"] == "review":
            self._second_review(number, status)
            if status not in ("infra", "interrupted", "failed"):
                self._settle(number)  # a comment left during the review gets its pass
            return
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

    def _closed(self, number: int, thread: dict[str, Any]) -> None:
        """Someone closed the issue or pull request while the run worked on it: its work is not
        pushed, no pull request opens, and nothing queues it again."""
        set_state_label(self.ctx, number, label_names(thread), None)
        where = "pull request" if "pull_request" in thread else "issue"
        self.gh.create_comment(number, f"This {where} was closed while a run was working on it "
                               f"({self._link()}), so that run's work was not delivered. Reopen "
                               f"it and ask again (`/harness build`) to have it done.")
        self.log.append(f"#{number} was closed during the run: nothing delivered")

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
        answered = not blocked and status in ("approved", "reviewed")
        asks.react(self.gh, settled, asks.DONE if answered else asks.NO_ANSWER)

    def _keep_handoff(self, number: int) -> None:
        """Keep what a run that did not finish the item leaves for the next agent: its notes
        and the end of its session (`work.py`). A finished item's handoff is dropped."""
        status = str(self.result.get("status"))
        handoff = self.result.get("handoff")
        finished = status in ("approved", "reviewed")
        if not finished and not isinstance(handoff, dict):
            return
        kept: dict[str, Any] | None = None
        if not finished and isinstance(handoff, dict):
            kept = {"provider": self.provider.id, "family": self.provider.family,
                    "at": str(handoff.get("at") or ""),
                    "reason": redact(str(self.result.get("reason") or ""))[:500],
                    "notes": redact(str(handoff.get("notes") or ""))[-HANDOFF_NOTES:],
                    "trail": redact(str(handoff.get("trail") or ""))[-HANDOFF_TRAIL:]}
        def change(state: dict[str, Any]) -> None:
            entry = state_item(state, number)
            if kept is None:
                entry.pop("handoff", None)
            else:
                entry["handoff"] = kept
        self.ctx.store.update(change, f"handoff #{number}")

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

    def _infra(self, number: int, kind: str, *, requeue: str | None = None) -> None:
        """A run that could not work. Its subscription is left alone for a while
        (`providers.INFRA_BACKOFF`), so the item goes to another one. When the model job reported
        why (its login, its CLI, an install on untouched main), that is all. When the job left no
        result at all (its runner died or timed out), it may be this item too: it goes to the back
        of the queue, and two in a row block it."""
        self.chain = False
        reason = str(self.result.get("reason") or NO_RESULT)
        now = iso(self.ctx.now())
        label = requeue or (LABEL_REVISE if kind == "revise" else LABEL_BUILD)
        self.ctx.store.update(lambda s: providers_mod.note_infra(
            s, self.provider.id, reason, self.ctx.now()), f"infra {self.provider.id}")
        where = f"on `{self.provider.id}`"
        if not self.result.get("no_result"):
            set_state_label(self.ctx, number, self._labels(number), label)
            self.gh.create_comment(number, f"The run could not work on this {where} "
                                   f"({self._link()}): {reason}\n\nThat is not this item's fault; "
                                   "it stays queued, for another subscription meanwhile.")
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
        set_state_label(self.ctx, number, self._labels(number), label)
        self.gh.create_comment(number, f"The run working on this died {where} ({self._link()}): "
                               f"{reason}. It is back in the queue, behind the others.")

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
            self.gh.add_labels(pr, [LABEL_PR, *self._carried(number)])
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
            if pr:
                self._carry_difficult(number, pr)
                rule = self._approved(pr, str(self.result.get("head") or ""),
                                      merge=not merge_note)
                merge_note = merge_note or rule
            cycles = len(self.result.get("cycles") or [])
            self.gh.create_comment(number, f"Opened #{pr}, built on {self.provider.describe()}. "
                                   f"Its adversarial reviewer approved it on round {cycles} of "
                                   f"{self.cfg.max_review_cycles}. {merge_note}")
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
        if bot_pr:
            pull = self.gh.get_pull(number)
            # The commit the reviewer approved: what was pushed, or with nothing pushed, where
            # the work started. Someone may have pushed meanwhile; that head nobody reviewed.
            reviewed = str(self.result.get("head") or "") if pushed else str(
                self.result.get("start") or "")
            now_head = self._branch_head(number)
            waiting = bool(self._record(number).get("pending_request"))
            if reviewed and now_head and reviewed != now_head:
                self._hold_auto_merge()
                rule = self._approved(number, now_head, merge=not waiting, vote=False)
            else:
                if pull.get("draft") and not waiting:
                    self._try(lambda: self.gh.mark_ready(pull["node_id"]))
                rule = self._approved(number, reviewed or now_head, merge=not waiting)
            merge_note = "" if waiting else "\n\n" + rule
        self.gh.create_comment(number, f"Revision {'pushed' if pushed else 'done'} "
                               f"({self._link()}); the adversarial reviewer approved it.{note}\n\n"
                               f"{report}{merge_note}")

    # ------------------------------------------------------------------ the two-model rule

    def _self_reviewing(self, family: str) -> bool:
        """True when a model family's own approval is enough to merge (Opus)."""
        return any(p.self_review for p in self.cfg.pool.ordered() if p.family == family)

    def _rule_met(self, votes: dict[str, Any]) -> bool:
        """A change merges when its exact head has its builder's model's approval and either
        that model is enough by itself or a second model approved too, and no model's rejection
        of that head stands (a model that rejected it and later approved it has withdrawn it)."""
        approvals = set(votes.get("approvals") or [])
        if set(votes.get("rejections") or []) - approvals:
            return False
        builder = str(votes.get("builder") or "")
        if builder:
            return builder in approvals and (self._self_reviewing(builder) or len(approvals) >= 2)
        return len(approvals) >= 2 or any(self._self_reviewing(f) for f in approvals)

    def _vote(self, pr: int, head: str, *, approve: bool, builder: bool) -> dict[str, Any]:
        """Record this run's verdict on `head`. A new head starts afresh: its builder is this
        run's model when this run built it, and unknown otherwise."""
        family = self.provider.family
        votes: dict[str, Any] = {}

        def change(state: dict[str, Any]) -> None:
            entry = state_item(state, pr)
            current = dict(entry.get("votes") or {})
            if not head or current.get("sha") != head:
                current = {"sha": head, "builder": family if builder else "", "approvals": [],
                           "rejections": []}
            key, other = ("approvals", "rejections") if approve else ("rejections", "approvals")
            current[key] = list(dict.fromkeys([*current.get(key, []), family]))
            current[other] = [f for f in current.get(other, []) if f != family]
            entry["votes"] = current
            votes.update(current)

        self.ctx.store.update(change, f"votes #{pr}")
        return votes

    def _approved(self, pr: int, head: str, *, merge: bool = True, vote: bool = True,
                  builder: bool = True) -> str:
        """Record this run's approval of `head` on the PR, then turn on auto-merge when the rule
        is met; otherwise turn it off and queue a second model's review. Returns the line for the
        comment."""
        if vote:
            votes = self._vote(pr, head, approve=True, builder=builder)
        else:
            votes = self._vote_reset(pr, head)
        if not merge:
            return ""
        labels = self._labels(pr)
        if LABEL_PR not in labels:
            return ""  # never auto-merge a pull request the bot did not open
        if self._rule_met(votes):
            return self._auto_merge(self.gh.get_pull(pr), expected_head=head)
        self._hold_auto_merge(pr)
        set_state_label(self.ctx, pr, labels, LABEL_CROSS)
        self._remember(pr, queued_at=iso(self.ctx.now()))
        who = (f"`{votes.get('builder')}` built and approved it" if votes.get("builder")
               else "Its head has not been approved by the model that built it")
        line = (f"{who}, so it waits for a second model's review (`{LABEL_CROSS}`) before "
                "auto-merge turns on.")
        if not self._other_family_set_up(str(votes.get("builder") or ""),
                                         low_tier_only=LABEL_SHITTER in {
                                             name.lower() for name in labels}):
            line += (" No subscription of another model family is set up, so it waits for you "
                     "to merge it, or for one to be set up.")
        return line

    def _vote_reset(self, pr: int, head: str) -> dict[str, Any]:
        """A head no model reviewed (someone pushed during the run): its votes start empty."""
        votes = {"sha": head, "builder": "", "approvals": [], "rejections": []}
        self._remember(pr, votes=votes)
        return votes

    def _other_family_set_up(self, family: str, low_tier_only: bool = False) -> bool:
        """Whether another model family may give the second review: for a `shitter` pull
        request, only a low-tier one (#96)."""
        secrets = self.cfg.secrets
        return any(p.enabled and p.family != family and "review" in p.roles
                   and (p.login == "machine" or secrets.has(p.secret) is not False)
                   and not (low_tier_only and providers_mod.model_tier(p.model)
                            == providers_mod.HIGH_TIER)
                   for p in self.cfg.pool.ordered())

    def _carried(self, issue: int) -> list[str]:
        """The issue's labels its pull request starts with, approved or a draft: `shitter`, so
        no high-tier model revises or second-reviews it (#96), and its priority tier (#90)."""
        return sorted(name for name in self._labels(issue)
                      if name.lower() in {LABEL_SHITTER, *PRIORITY_TIERS})

    def _carry_difficult(self, issue: int, pr: int) -> None:
        """A `difficult` issue's pull request stays Opus-only for its revisions."""
        label = self.cfg.pool.difficult_label
        if label in self._labels(issue):
            self.gh.add_labels(pr, [label])
            self._remember(pr, difficult=True)

    def _second_review(self, number: int, status: str) -> None:
        """Act on a second model's verdict on a bot pull request."""
        record = self._record(number)
        if status == "infra":
            self._infra(number, "review", requeue=LABEL_CROSS)
            return
        if status == "interrupted":
            set_state_label(self.ctx, number, self._labels(number), LABEL_CROSS)
            self.gh.create_comment(number, f"The second review was cut short ({self._link()}): "
                                   f"{self.result.get('reason')}. It stays queued.")
            return
        if status == "stopped":
            set_state_label(self.ctx, number, self._labels(number), None)
            self.gh.create_comment(number, "Stopped, as asked. It will not merge by itself.")
            return
        if status != "reviewed":
            self._fail_review(number, record)
            return
        head = self._branch_head(number)
        reviewed = str(self.result.get("reviewed_sha") or "")
        who = self.provider.describe()
        if reviewed != head:
            set_state_label(self.ctx, number, self._labels(number), LABEL_CROSS)
            self.gh.create_comment(number, f"The branch moved while {who} reviewed it, so that "
                                   "review does not count. It stays queued for a second review.")
            return
        if self.result.get("verdict") == "approve":
            set_state_label(self.ctx, number, self._labels(number), None)
            self._remember(number, cross_rounds=0, failures=0)
            waiting = bool(record.get("pending_request"))
            note = self._approved(number, head, merge=not waiting, builder=False)
            if waiting:
                note = "A comment arrived during the review, so a revision answers it first."
            self.gh.create_comment(number, f"Second review ({self._link()}): {who} approved it."
                                   f" {note}")
            return
        # A rejection stands against this head until that model approves it or the head moves.
        self._vote(number, head, approve=False, builder=False)
        self._hold_auto_merge(number)
        rounds = int(record.get("cross_rounds", 0)) + 1
        findings = self.result.get("findings") or []
        if rounds >= self.cfg.max_failures:
            set_state_label(self.ctx, number, self._labels(number), LABEL_BLOCKED)
            self._remember(number, cross_rounds=rounds, last_findings=findings)
            self.gh.create_comment(number, f"Second review ({self._link()}): {who} still found "
                                   f"blocking problems, after {rounds} rounds between the models, "
                                   f"so I stopped. It needs a person.\n\n{self._findings_md()}")
            return
        set_state_label(self.ctx, number, self._labels(number), LABEL_REVISE)
        self._remember(number, cross_rounds=rounds, last_findings=findings, source="cross-review",
                       kind="revise", queued_at=iso(self.ctx.now()), failures=0)
        self.gh.create_comment(number, f"Second review ({self._link()}): {who} found blocking "
                               f"problems, so a revision is queued to answer them.\n\n"
                               f"{self._findings_md()}")

    def _branch_head(self, number: int) -> str:
        """The pull request's head as git sees it on origin now (the API can lag a push)."""
        branch = str(self.plan.get("branch") or "")
        if branch:
            self.repo.run("fetch", "--quiet", "--no-tags", "origin",
                          f"+refs/heads/{branch}:refs/remotes/origin/{branch}", check=False)
            found = self.repo.rev(f"origin/{branch}")
            if found:
                return found
        return str((self.gh.get_pull(number).get("head") or {}).get("sha") or "")

    def _fail_review(self, number: int, record: dict[str, Any]) -> None:
        failures = int(record.get("failures", 0)) + 1
        self._remember(number, failures=failures)
        if failures >= self.cfg.max_failures:
            set_state_label(self.ctx, number, self._labels(number), LABEL_BLOCKED)
            self.gh.create_comment(number, f"The second review failed {failures} times "
                                   f"({self._link()}): {self.result.get('reason')}. It needs a "
                                   "person.")
            return
        set_state_label(self.ctx, number, self._labels(number), LABEL_CROSS)
        self.gh.create_comment(number, f"The second review failed ({self._link()}): "
                               f"{self.result.get('reason')}. It stays queued.")

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

    def _auto_merge(self, pull: dict[str, Any], expected_head: str = "") -> str:
        """Turn on auto-merge only when nothing needs a person and CI must pass first, pinned to
        `expected_head` (the approved commit) when given."""
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
        error = self._try(lambda: self.gh.enable_auto_merge(pull["node_id"], self.cfg.merge_method,
                                                            expected_head))
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
                s["suggest"].update(before, provider=None, run_id=None)
                asks.give_back(s["suggest"])

            self.ctx.store.update(unfinished, "survey unfinished")
        else:
            settled: list[str] = []

            def done(s: dict[str, Any]) -> None:
                s["suggest"].update(provider=None, run_id=None)
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
        """Start the next run at once when a lane is free and a subscription can take more."""
        if not self.chain:
            return
        look = plan_mod.peek(self.ctx)
        if look.work:
            self.ctx.dispatch()
            self.log.append(f"dispatched the next run: {look.reason}")
