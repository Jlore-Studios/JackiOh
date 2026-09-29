"""The last job of a night run: check the model job's output and publish it.

This job holds the bot's token and runs no model. It does not trust the model job's files: the
bundle must hold the branch head the result names, descend from the commit the work started at,
leave every forbidden path alone, and fast-forward the branch on GitHub. A pull request the
reviewer approved gets auto-merge, which merges it once the required CI checks pass.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from harness import gates as gates_mod
from harness.clock import iso
from harness.config import (LABEL_BLOCKED, LABEL_BUILD, LABEL_NEEDS_REVIEW, LABEL_PR,
                            LABEL_PR_OPEN, LABEL_REVISE, LABEL_SUGGESTION, LABEL_WORKING)
from harness.context import Context
from harness.errors import GitError, GitHubError
from harness.git import Git, is_forbidden, matches
from harness.plan import suggestions_due
from harness.queue import candidates, label_names, open_pull_for_branch, set_state_label
from harness.state import item as state_item
from harness.state import record_usage, usage_refusal

REPORT_CHARS = 30_000


def load_result(out_dir: Path) -> dict[str, Any]:
    path = Path(out_dir) / "result.json"
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
        if isinstance(data, dict):
            return data
    except (OSError, ValueError):
        pass
    return {"status": "failed", "reason": "the model job left no result (it was cancelled, "
            "timed out or crashed before writing one)"}


class Deliverer:
    def __init__(self, ctx: Context, plan: dict[str, Any], out_dir: Path, repo_dir: Path) -> None:
        self.ctx = ctx
        self.cfg = ctx.cfg
        self.gh = ctx.gh
        self.plan = plan
        self.out_dir = Path(out_dir)
        self.repo = Git(repo_dir)
        self.result = load_result(out_dir)
        self.log: list[str] = []

    # ------------------------------------------------------------------ entry

    def run(self) -> dict[str, Any]:
        action = self.plan.get("action")
        usage, reset_at = self.result.get("usage"), self.result.get("reset_at")
        self.ctx.store.update(lambda s: record_usage(s, usage, reset_at, self.ctx.now()), "usage")
        if action == "suggest":
            self._suggestions()
        elif action in ("build", "revise"):
            self._item()
        self._chain()
        return {"status": self.result.get("status"), "log": self.log}

    # ------------------------------------------------------------------ the branch

    def _publish(self) -> tuple[bool, str]:
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
        if start and self.repo.run("merge-base", "--is-ancestor", start, head,
                                   check=False).returncode != 0:
            return False, "the bundle does not descend from the commit the work started at"
        remote = self.repo.rev(f"origin/{branch}")
        if remote and remote != start:
            return False, (f"`{branch}` moved on GitHub while I worked, so I did not overwrite it")
        changed = self.repo.changed_paths(f"origin/{default}", local)
        forbidden = [p for p in changed if is_forbidden(p, self.cfg.forbidden_paths)]
        if forbidden:
            return False, f"the change touches paths the bot may not change: {', '.join(forbidden)}"
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

    def _item(self) -> None:
        number = int(self.plan["number"])
        status = str(self.result.get("status"))
        if self.plan["action"] == "revise":
            self._revise(number, status)
        else:
            self._build(number, status)

    def _labels(self, number: int) -> set[str]:
        return label_names(self.gh.get_issue(number))

    def _remember(self, number: int, **fields: Any) -> None:
        self.ctx.store.update(lambda s: state_item(s, number).update(fields), f"deliver #{number}")

    def _fail(self, number: int, kind: str) -> None:
        """A run that produced nothing usable: requeue it, or block it after too many."""
        state = self.ctx.store.update(
            lambda s: state_item(s, number).update(
                failures=int(state_item(s, number).get("failures", 0)) + 1),
            f"failure #{number}")
        failures = int(state_item(state, number).get("failures", 0))
        reason = self.result.get("reason") or "unknown"
        link = f" ([run]({self.cfg.run_url}))" if self.cfg.run_url else ""
        if failures >= self.cfg.max_failures:
            set_state_label(self.ctx, number, self._labels(number), LABEL_BLOCKED)
            self.gh.create_comment(number, f"This failed {failures} times in a row{link}, so I "
                                   f"stopped trying. Last error: {reason}\n\nFix the cause, then "
                                   "`/harness build` (or `revise`) queues it again.")
        else:
            wanted = LABEL_REVISE if kind == "revise" else LABEL_BUILD
            set_state_label(self.ctx, number, self._labels(number), wanted)
            self.gh.create_comment(number, f"This run failed{link}: {reason}\n\nIt is back in the "
                                   f"queue (attempt {failures} of {self.cfg.max_failures}).")

    def _build(self, number: int, status: str) -> None:
        link = f"[run]({self.cfg.run_url})" if self.cfg.run_url else "the run"
        if status in ("failed", "nothing"):
            if status == "nothing":
                set_state_label(self.ctx, number, self._labels(number), None)
                return
            self._fail(number, "build")
            return
        if status == "stopped":
            pushed, _ = self._publish()
            kept = f" What I had is on `{self.plan['branch']}`." if pushed else ""
            self.gh.create_comment(number, f"Stopped, as asked.{kept}")
            return
        if status == "interrupted":
            pushed, problem = self._publish()
            set_state_label(self.ctx, number, self._labels(number), LABEL_BUILD)
            self._remember(number, last_findings=self.result.get("findings") or [])
            kept = " The work so far is on the branch." if pushed else ""
            self.gh.create_comment(number, f"Paused: {self.result.get('reason')}.{kept} It stays "
                                   "queued and the next run picks it up where this one stopped."
                                   + (f"\n\n{problem}" if problem else ""))
            return
        pushed, problem = self._publish()
        if problem:
            set_state_label(self.ctx, number, self._labels(number), LABEL_BLOCKED)
            self.gh.create_comment(number, f"I could not publish this run's work ({link}): "
                                   f"{problem}. It needs a person to look.")
            return
        branch = str(self.plan["branch"])
        if status == "blocked":
            set_state_label(self.ctx, number, self._labels(number), LABEL_BLOCKED)
            question = self.result.get("question") or self.result.get("reason")
            kept = f" What I had is on `{branch}`." if pushed else ""
            self.gh.create_comment(number, f"I need a decision before I can build this.{kept}\n\n"
                                   f"{question}\n\nAnswer here, then `/harness build` (or "
                                   f"`@{self.cfg.bot_login} <your answer>`) queues it again.")
            self._remember(number, last_findings=[])
            return
        approved = status == "approved"
        remote = self.repo.rev(f"origin/{branch}") or pushed
        if not remote:
            self._fail(number, "build")
            return
        body = self._pull_body(number, approved)
        title = str(self.result.get("title") or self.plan.get("title") or f"Build #{number}")
        pull = open_pull_for_branch(self.ctx, branch)
        if pull is None:
            pull = self.gh.create_pull(title=title, head=branch, base=self.cfg.default_branch,
                                       body=body, draft=not approved)
        else:
            self.gh.update_pull(int(pull["number"]), title=title, body=body)
            if approved and pull.get("draft"):
                self._try(lambda: self.gh.mark_ready(pull["node_id"]))
        pr = int(pull.get("number") or 0)
        if pr:
            self.gh.add_labels(pr, [LABEL_PR])
        issue_labels = self._labels(number)
        if approved:
            set_state_label(self.ctx, number, issue_labels, None)
            if LABEL_PR_OPEN not in issue_labels:
                self.gh.add_labels(number, [LABEL_PR_OPEN])
            merge_note = self._auto_merge(pull)
            if pr:
                set_state_label(self.ctx, pr, label_names(self.gh.get_issue(pr)), None)
            cycles = len(self.result.get("cycles") or [])
            self.gh.create_comment(number, f"Opened #{pr}. The adversarial reviewer approved it on "
                                   f"round {cycles} of {self.cfg.max_review_cycles}. {merge_note}")
            self._remember(number, last_findings=[], failures=0, pr=pr, last_push_at=iso(self.ctx.now()))
            if pr:
                self._remember(pr, issue=number, last_push_at=iso(self.ctx.now()), failures=0)
        else:
            set_state_label(self.ctx, number, issue_labels, LABEL_BLOCKED)
            if pr:
                set_state_label(self.ctx, pr, label_names(self.gh.get_issue(pr)), LABEL_BLOCKED)
            findings = self._findings_md()
            self.gh.create_comment(number, f"After {self.cfg.max_review_cycles} rounds the "
                                   f"reviewer still had blocking findings, so #{pr} stays a "
                                   f"draft and will not merge by itself ({link}).\n\n{findings}\n\n"
                                   f"Comment `@{self.cfg.bot_login} <guidance>` here or on #{pr} "
                                   "to have me try again with your notes.")
            self._remember(number, last_findings=self.result.get("findings") or [], pr=pr,
                           last_push_at=iso(self.ctx.now()))

    def _revise(self, number: int, status: str) -> None:
        link = f"[run]({self.cfg.run_url})" if self.cfg.run_url else "the run"
        bot_pr = bool(self.plan.get("bot_pr"))
        if status == "failed":
            self._fail(number, "revise")
            return
        if status == "stopped":
            self.gh.create_comment(number, "Stopped, as asked. I pushed nothing.")
            return
        if status == "interrupted":
            set_state_label(self.ctx, number, self._labels(number), LABEL_REVISE)
            self.gh.create_comment(number, f"Paused: {self.result.get('reason')}. The revision "
                                   "stays queued; the next run starts it again.")
            return
        if status == "blocked":
            set_state_label(self.ctx, number, self._labels(number), LABEL_BLOCKED)
            question = self.result.get("question") or self.result.get("reason")
            self.gh.create_comment(number, f"I need a decision before I can revise this.\n\n"
                                   f"{question}")
            return
        if status != "approved":
            set_state_label(self.ctx, number, self._labels(number), LABEL_BLOCKED)
            self.gh.create_comment(number, f"My revision did not pass review after "
                                   f"{self.cfg.max_review_cycles} rounds, so I pushed nothing "
                                   f"({link}).\n\n{self._findings_md()}")
            self._remember(number, last_findings=self.result.get("findings") or [])
            return
        pushed, problem = self._publish()
        if problem:
            set_state_label(self.ctx, number, self._labels(number), LABEL_BLOCKED)
            self.gh.create_comment(number, f"I could not push the revision ({link}): {problem}.")
            return
        set_state_label(self.ctx, number, self._labels(number), None)
        report = str(self.result.get("report") or "")[:REPORT_CHARS]
        note = ""
        if pushed:
            self._remember(number, last_push_at=iso(self.ctx.now()), failures=0, source="",
                           last_findings=[])
        else:
            note = "\n\nI changed nothing: the reviewer agreed no change was needed."
            if self.plan.get("source") == "ci":
                note += " I asked CI to run the failed jobs again."
                self._rerun_ci(number)
            self._remember(number, failures=0, source="")
        merge_note = ""
        if bot_pr:
            merge_note = "\n\n" + self._auto_merge(self.gh.get_pull(number))
        self.gh.create_comment(number, f"Revision {'pushed' if pushed else 'done'} ({link}); the "
                               f"adversarial reviewer approved it.{note}\n\n{report}{merge_note}")

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
        local = f"deliver/{branch}"
        ref = local if self.repo.rev(local) else f"origin/{branch}"
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
        return self._hand_to_a_person(pull, f"GitHub refused it ({error})")

    def _rerun_ci(self, number: int) -> None:
        state = self.ctx.store.load()
        run_id = state_item(state, number).get("ci_run_id")
        if run_id:
            self._try(lambda: self.gh.rerun_failed_jobs(run_id))

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
        if not ctx.window.is_open(now) or ctx.repo_halted():
            return
        state = ctx.store.load()
        if state.get("halted") or usage_refusal(state, dict(self.cfg.usage_stop), now):
            return
        if candidates(ctx, state) or suggestions_due(ctx, state):
            ctx.dispatch()
            self.log.append("dispatched the next run")
