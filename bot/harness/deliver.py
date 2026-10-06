"""The last job of a night run: check the model job's output and publish it.

This job holds the bot's token and runs no model. It does not trust the model job's files: the
item comes from the plan job's outputs and the branch is worked out again here, and the bundle
must hold the head the result names, descend from the commit the work started at, change no
forbidden path, and fast-forward the branch on GitHub.

The review rule (`review_rule.py`): a commit ships when one strong model (Opus) approved it, or
two medium models did (the same model twice included), or, for an easy item, one weak model and
one medium one did, and no model's rejection of it stands. Votes are kept per commit, one per
review, with the tier of the seat that cast them (`_vote`), from the plan's seats, never from the
model job. A pull request whose commit has that gets auto-merge, which merges it once the
required CI checks pass; one still short of it waits in `bot:cross-review` for a review run.

A commit that meets the rule is recorded as cleared (`_clear`). When `main` then leaves it with
conflicts, the revision that resolves them carries the clearance (`_carry`) if it started from
that commit, its own run's reviewer approved it, and git shows it changed nothing beyond the files
the merge left conflicted; it then ships on that one approval, with no review run.
"""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any

from harness import asks, disk, failures, issueplan, memory, review_rule
from harness import easy as easy_mod
from harness import journal as journal_mod
from harness import stepup
from harness import gates as gates_mod
from harness import plan as plan_mod
from harness import providers as providers_mod
from harness import vault
from harness.clock import iso, parse_iso
from harness.config import (DIFFICULTIES, DIFFICULTY_LABELS, LABEL_APPROVED, LABEL_BLOCKED,
                            LABEL_BUILD,
                            LABEL_CROSS, PLAN_FLOOR,
                            LABEL_HUMAN, LABEL_NEEDS_PLAN, LABEL_PLANNED, LABEL_STUCK,
                            LABEL_NEEDS_REVIEW, LABEL_PR, LABEL_PR_OPEN, LABEL_READY,
                            LABEL_REVISE, LABEL_SUGGESTION, STATE_BRANCH)
from harness.context import Context
from harness.errors import GitError, GitHubError
from harness.git import Git, matches
from harness.queue import (PRIORITY_TIERS, branch_for_issue, cleared, difficulty_of, label_names,
                           labelled_difficulty, open_pull_for_branch, rating_source,
                           set_state_label, wip_branch)
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


def _cut(text: str, limit: int, *, head: bool) -> str:
    """`text` cut to `limit` characters: its beginning (a plan) or its end (a builder's log)."""
    return text[:limit] if head else text[-limit:]


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
        seats = self.plan.get("seats") if isinstance(self.plan.get("seats"), dict) else {}
        main = pool.seats(self.provider)[0]

        def seat(role: str) -> Any:
            entry = seats.get(role) if isinstance(seats.get(role), dict) else {}
            return pool.seat(self.provider.id, str(entry.get("model") or "")) if entry else None
        #: The seats that built and reviewed in the run, from the plan's outputs: a vote's tier
        #: comes from here, never from the model job's result.
        self.build_seat = seat("build") or main
        self.review_seat = seat("review") if seats else main
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
        self._machine_disk()
        self._machine_memory()
        self._login_works()
        if action == "suggest":
            self._suggestions()
        elif action in ("plan", "build", "revise", "review"):
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

    def _machine_disk(self) -> None:
        """A job on the bot's machine read its disk: keep the newest reading, and open, rewrite
        or close the issue about a disk filling up (`disk.py`)."""
        if providers_mod.hosted(self.provider.runs_on):
            return
        found = disk.newest(self.result)
        if found is None:
            return
        reported = str((self.result.get("disk") or {}).get("runner") or "")
        runner = reported if re.fullmatch(r"[\w.-]{1,80}", reported) else self.provider.runs_on
        self.ctx.store.update(lambda s: disk.note(s, found, runner, self.cfg.run_url or ""),
                              "machine disk")
        try:
            done = disk.alert(self.ctx)
        except GitHubError as exc:
            done = f"could not settle the disk issue: {exc}"
        if done:
            self.log.append(done)

    def _machine_memory(self) -> None:
        """A job on the bot's machine read its memory: keep its summary for the status issue's
        line on the machine's memory (`memory.py`, #312)."""
        if providers_mod.hosted(self.provider.runs_on):
            return
        found = memory.summary(self.result.get("memory"))
        if found is None:
            return
        reported = str((self.result.get("disk") or {}).get("runner") or "")
        runner = reported if re.fullmatch(r"[\w.-]{1,80}", reported) else self.provider.runs_on
        self.ctx.store.update(lambda s: memory.note(s, found, runner), "machine memory")

    def _login_works(self) -> None:
        """A model call went through on this run's subscription: any streak of runs that could
        not work there is over (`providers.clear_infra`), and the issue asking a person about it,
        if one was opened, is closed."""
        try:
            calls = int(self.result.get("model_calls") or 0)
        except (TypeError, ValueError):
            calls = 0
        if self.result.get("status") == "infra" or not calls:
            return
        if not isinstance(providers_mod.peek_record(self.ctx.store.load(),
                                                    self.provider.id).get("infra"), dict):
            return
        opened: list[int] = []
        self.ctx.store.update(lambda s: opened.extend(
            n for n in [providers_mod.clear_infra(s, self.provider.id)] if n),
            f"{self.provider.id} works again")
        for number in opened:
            self._try(lambda n=number: self.gh.create_comment(
                n, f"`{self.provider.id}` works again: a run got a model call through "
                   f"({self._link()}). Closing this."))
            self._try(lambda n=number: self.gh.update_issue(n, state="closed"))
            self.log.append(f"{self.provider.id} works again: closed #{number}")

    def _ask_about_infra(self, streak: int, reason: str) -> None:
        """After `providers.INFRA_ASK_AFTER` runs in a row that could not work on this
        subscription, open one issue asking a person to fix it (the bot cannot: a login is a
        secret, and `.harness/` is not the bot's to change)."""
        if streak < providers_mod.INFRA_ASK_AFTER:
            return
        record = providers_mod.peek_record(self.ctx.store.load(), self.provider.id)
        if (record.get("infra") or {}).get("issue"):
            return
        backoff = providers_mod.INFRA_BACKOFFS[-1]
        hours = int(backoff.total_seconds() // 3600)
        login = self.provider.secret or "its login on the machine"
        body = (f"The last {streak} runs on `{self.provider.id}` ({self.provider.cli}, "
                f"`{self.provider.model}`) could not work. The last one ({self._link()}) said:\n\n"
                f"> {reason[:500]}\n\nUntil a run on it gets a model call through, unforced runs "
                f"try it only once every {hours} hours, and the other subscriptions take its "
                "work.\n\nTo fix it, a person (the bot cannot change secrets or `.harness/`):\n"
                f"- renews its login (`{login}`), if the reason is a refused or expired login;\n"
                "- or switches it off (`\"enabled\": false` in `.harness/providers.json`).\n\n"
                f"The bot closes this issue by itself once a run on `{self.provider.id}` works "
                "again.")
        try:
            issue = self.gh.create_issue(
                f"Night bot: `{self.provider.id}` could not work {streak} times in a row", body,
                labels=["night bot", LABEL_HUMAN])
        except GitHubError as exc:
            self.log.append(f"could not open an issue about {self.provider.id}: {exc}")
            return
        number = int(issue.get("number") or 0)
        if number:
            self.ctx.store.update(lambda s: providers_mod.record(s, self.provider.id).setdefault(
                "infra", {}).update(issue=number), f"{self.provider.id} infra issue")
            self.log.append(f"{self.provider.id} could not work {streak} times: opened #{number}")

    def _rederive(self) -> None:
        """Work the branch out again from GitHub, whatever the plan file says."""
        number = int(self.plan["number"])
        if self.plan["action"] in ("plan", "build"):
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
        """Push the bundle's branch. `(pushed, problem)`; no bundle is `(False, "")`. A bundle
        whose head holds a conflict marker in a file it changed is refused (#317 part 2)."""
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
        markers = self.repo.markers(local, self.repo.changed_paths(f"origin/{default}", local))
        if markers:
            return False, ("conflict markers are still in "
                           + ", ".join(f"`{p}`" for p in markers[:8])
                           + (", …" if len(markers) > 8 else "")
                           + ", so I pushed nothing")
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
        if status in ("approved", "built", "not_approved", "blocked") and (
                state.get("halted") or self.ctx.repo_halted()):
            self.result.update(status="interrupted", interrupt="halt",
                               reason="the bot was halted before this work could be published")
            return "interrupted"
        return status

    def _item(self) -> None:
        number = int(self.plan["number"])
        self.rating_note = ""
        if self.plan.get("action") in ("plan", "build") and isinstance(self.result.get("plan"), dict):
            self.rating_note = self._apply_rating(number)
        if self.plan.get("action") == "build" and isinstance(self.result.get("plan"), dict):
            # The run planned it first: a later run builds from that plan, never plans again.
            who, tier = self._planner()
            self._remember(number, planned_at=iso(self.ctx.now()), planned_by=who,
                           planned_tier=tier)
            self._plan_into_issue(number, who)
            if self.rating_note and self.result.get("status") != "planned":
                self._try(lambda: self.gh.create_comment(number, self.rating_note))
        self._deliver_item(number)
        self._keep_handoff(number)
        self._journal(number)
        self._close_asks(number)

    def _deliver_item(self, number: int) -> None:
        thread = self.gh.get_issue(number)
        if thread.get("state") != "open":
            self._closed(number, thread)
            return
        status = self._late_stop(number, str(self.result.get("status")))
        if self.plan["action"] == "plan" or (self.plan["action"] == "build"
                                             and status == "planned"):
            # A build run that rated the item out of its reach stopped after planning it.
            self._planned(number, status)
            return
        if self.plan["action"] == "review":
            self._second_review(number, status)
            if status not in ("infra", "interrupted", "failed"):
                self._settle(number)  # a comment left during the review gets its pass
            return
        kind = "revise" if self.plan["action"] == "revise" else "build"
        if status == "infra":
            if not self.result.get("no_result"):
                # A login or CLI that broke mid-run (#317 part 1) leaves the work it had, like a
                # pause does: the next run, on another subscription, goes on from it.
                self._keep_work(number, kind)
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

    def _planned(self, number: int, status: str) -> None:
        """A planning run's end: its plan is the item's handoff (`_keep_handoff`), and the item
        goes back to the queue to build from it on the cheapest model its difficulty allows."""
        if status == "infra":
            self._infra(number, "build")
            return
        if status == "stopped":
            set_state_label(self.ctx, number, self._labels(number), None)
            self.gh.create_comment(number, "Stopped, as asked, before any plan was written.")
            return
        if status != "planned":
            if status == "interrupted":
                self._requeue_label(number, "build")
                self.gh.create_comment(number, f"Planning was cut short ({self._link()}): "
                                       f"{self.result.get('reason')}. It stays queued.")
                return
            self._fail(number, "build")
            return
        who, tier = self._planner()
        self._remember(number, planned_at=iso(self.ctx.now()), planned_by=who, planned_tier=tier)
        self._requeue_label(number, "build")
        rated = getattr(self, "rating_note", "")
        rated = f"{rated} " if rated else ""
        difficulty = self._difficulty(number)
        if tier and not providers_mod.tier_at_least(tier, PLAN_FLOOR[difficulty]):
            # A medium planner rated it medium or hard: a strong model plans it next (part 6).
            nxt = (f"A {PLAN_FLOOR[difficulty]} model plans `difficulty:{difficulty}` work, so "
                   "one plans it next; it is queued for that.")
        else:
            nxt = ("Cleared and queued to build, on the cheapest model "
                   f"`difficulty:{difficulty}` allows.")
        if self._plan_into_issue(number, who):
            self.gh.create_comment(number, f"{rated}Planned on {who} ({self._link()}). The plan "
                                   "is in this issue's description, under **Plan**: the builder "
                                   "starts from that section, so edit it there to change the "
                                   f"plan. {nxt}")
            return
        plan = self.result.get("plan") if isinstance(self.result.get("plan"), dict) else {}
        text = redact(str(plan.get("text") or ""))[:issueplan.PLAN_CHARS]
        self.gh.create_comment(number, f"Planned on {who} ({self._link()}), but the description "
                               "could not take the plan, so it is here. It is queued to build "
                               "from this plan, on the cheapest model its difficulty allows.\n\n"
                               f"<details><summary>The plan</summary>\n\n{text}\n\n</details>")

    def _apply_rating(self, number: int) -> str:
        """Label the item with the difficulty its planner rated (#317 part 8), and say so: only
        when no person set one (`queue.rating_source`), never lower than the bot's own earlier
        rating, and easy only when the plan's **Files to touch** table holds to the easy rule
        (`easy.plan_breach`). Records `difficulty_by`. Returns the sentence for the comment."""
        plan = self.result.get("plan") if isinstance(self.result.get("plan"), dict) else {}
        rated = plan.get("rating") if isinstance(plan.get("rating"), dict) else None
        if not rated or str(rated.get("difficulty")) not in DIFFICULTIES:
            return ""
        labels = self._labels(number)
        record = self._record(number)
        if rating_source(labels, record) == "person":
            return ""
        wanted = str(rated["difficulty"])
        earlier = str((record.get("difficulty_by") or {}).get("difficulty") or "")
        if earlier in DIFFICULTIES and DIFFICULTIES.index(wanted) < DIFFICULTIES.index(earlier):
            wanted = earlier  # a rating stands: nobody lowers it
        breach = ""
        if wanted == "easy":
            breach = easy_mod.plan_breach(str(plan.get("text") or ""), self.cfg.easy,
                                          self.cfg.review_paths, self.cfg.forbidden_paths)
            if breach:
                wanted = "medium"
        who, tier = self._planner()
        self._set_difficulty(number, wanted, labels)
        self._remember(number, difficulty_by={"difficulty": wanted, "provider": self.provider.id,
                                              "tier": tier, "at": iso(self.ctx.now()),
                                              "why": str(rated.get("why") or "")[:300]})
        why = f": {rated.get('why')}" if rated.get("why") else ""
        said = f"Rated `difficulty:{rated['difficulty']}` by {who}{why}."
        if breach:
            said += (f" The easy rule does not hold ({breach}), so it is "
                     "`difficulty:medium`.")
        elif wanted != rated["difficulty"]:
            said += f" It was rated `difficulty:{wanted}` before, and a rating is never lowered."
        return said

    def _set_difficulty(self, number: int, difficulty: str, labels: set[str]) -> None:
        """Leave exactly `difficulty:<difficulty>` on the thread, of the difficulty labels."""
        wanted = f"difficulty:{difficulty}"
        for name in labels:
            if name.lower() in DIFFICULTY_LABELS and name != wanted:
                self._try(lambda n=name: self.gh.remove_label(number, n))
        if wanted not in labels:
            self._try(lambda: self.gh.add_labels(number, [wanted]))

    def _planner(self) -> tuple[str, str]:
        """Who wrote this run's plan, and that model's tier."""
        plan = self.result.get("plan") if isinstance(self.result.get("plan"), dict) else {}
        seat = plan.get("seat") if isinstance(plan.get("seat"), dict) else {}
        planner = self.cfg.pool.seat(self.provider.id, str(seat.get("model") or ""))
        if planner is None:
            return self.provider.describe(), str(seat.get("tier") or "")
        return planner.describe(), planner.tier

    def _plan_into_issue(self, number: int, who: str) -> bool:
        """Put this run's plan into the issue's description, in place of an earlier one, and
        take the item out of the Needs plan stage. False when the description could not take it."""
        plan = self.result.get("plan") if isinstance(self.result.get("plan"), dict) else {}
        text = redact(str(plan.get("text") or "")).strip()
        if not text:
            return False
        try:
            thread = self.gh.get_issue(number)
            self.gh.set_issue_body(number, issueplan.with_plan(thread.get("body"), text, who,
                                                               self._link()))
        except GitHubError:
            return False
        labels = self._labels(number)
        _, tier = self._planner()
        # A plan built from in the same run stands; one that stopped there must meet its floor.
        planning_only = self.plan.get("action") == "plan" or self.result.get("status") == "planned"
        meets = (not planning_only or not tier
                 or providers_mod.tier_at_least(tier, PLAN_FLOOR[self._difficulty(number)]))
        try:
            if meets:
                if LABEL_NEEDS_PLAN in labels:
                    self.gh.remove_label(number, LABEL_NEEDS_PLAN)
                if LABEL_PLANNED not in labels:
                    self.gh.add_labels(number, [LABEL_PLANNED])
            elif LABEL_NEEDS_PLAN not in labels:
                # A medium plan of a medium or hard item (#317 part 6): a strong one comes next.
                self.gh.add_labels(number, [LABEL_NEEDS_PLAN])
        except GitHubError:
            pass  # the next plan job's sync puts the labels right
        return True

    def _closed(self, number: int, thread: dict[str, Any]) -> None:
        """Someone closed the issue or pull request while the run worked on it: its work is not
        pushed, no pull request opens, and nothing queues it again."""
        set_state_label(self.ctx, number, label_names(thread), None)
        if "pull_request" in thread:
            self._drop_wip(number)
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
        again = not blocked and status in ("interrupted", "infra", "failed", "planned")
        settled: list[str] = []

        def change(state: dict[str, Any]) -> None:
            entry = state_item(state, number)
            if again:
                asks.give_back(entry)
            else:
                settled[:] = asks.pop_taken(entry)

        self.ctx.store.update(change, f"asks #{number}")
        answered = not blocked and status in ("approved", "built", "reviewed")
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
            notes = redact(str(handoff.get("notes") or ""))
            kept = {"provider": self.provider.id, "family": self.provider.family,
                    "kind": "plan" if status == "planned" else "work",
                    "at": str(handoff.get("at") or ""),
                    "reason": redact(str(self.result.get("reason") or ""))[:500],
                    "notes": (_cut(notes, HANDOFF_NOTES, head=True) if status == "planned"
                              else journal_mod.compact(notes, HANDOFF_NOTES)),
                    "trail": redact(str(handoff.get("trail") or ""))[-HANDOFF_TRAIL:]}
        def change(state: dict[str, Any]) -> None:
            entry = state_item(state, number)
            if kept is None:
                entry.pop("handoff", None)
            else:
                entry["handoff"] = kept
        self.ctx.store.update(change, f"handoff #{number}")

    def _journal(self, number: int) -> None:
        """Append this run to the item's journal on `bot-journal` (#342): what it was, each step
        the work job took and why, how it ended, and the builder's notes. A record, never a gate:
        a failure is logged and the delivery stands."""
        key = journal_mod.key(self.plan) or number
        seats = [f"`{s.model}` ({s.tier})" for s in (self.build_seat, self.review_seat) if s]
        models = ", ".join(dict.fromkeys(seats)) or self.provider.cli
        text = journal_mod.section(
            self.result, provider=self.provider.id, models=models,
            action=str(self.plan.get("action") or "?"), number=number, at=iso(self.ctx.now()),
            run_url=self.cfg.run_url)
        if self.cfg.dry_run:
            self.log.append(f"dry run: would add this run to the journal of #{key}")
            return
        try:
            journal_mod.append(self.gh, key, text)
            self.log.append(f"added this run to the journal of #{key}")
        except Exception as exc:  # noqa: BLE001 - the delivery already stands
            self.log.append(f"the journal of #{key} could not be written: {redact(str(exc))}")

    def _journal_link(self, number: int) -> str:
        key = journal_mod.key(self.plan) or number
        return f"[journal]({journal_mod.url(self.cfg.server_url, self.cfg.repo, key)})"

    def _labels(self, number: int) -> set[str]:
        return label_names(self.gh.get_issue(number))

    def _record(self, number: int) -> dict[str, Any]:
        return dict(self.ctx.store.load()["items"].get(str(number), {}))

    def _remember(self, number: int, **fields: Any) -> None:
        self.ctx.store.update(lambda s: state_item(s, number).update(fields), f"deliver #{number}")

    def _link(self) -> str:
        return f"[run]({self.cfg.run_url})" if self.cfg.run_url else "the run"

    def _built_on(self) -> str:
        """The builder's seat, and every move the run made between its lane's models (each
        Claude account switches between Opus and Sonnet: `work.Worker._switch`)."""
        moves = []
        for move in self.result.get("switches") or []:
            if not isinstance(move, dict):
                continue
            when = f"after round {move['n']}" if move.get("n") else "before round 1"
            moves.append(f"{when} it switched to `{move.get('to')}`: {move.get('why')}")
        text = self.build_seat.describe()
        return text + (f" ({'; '.join(moves)})" if moves else "")

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
        # A failure that was not the subscription's (an install red on untouched main, a push
        # GitHub refused) backs it off but does not lengthen its streak.
        escalate = not self.result.get("infra_scope")
        streak: list[int] = []
        self.ctx.store.update(lambda s: streak.append(providers_mod.note_infra(
            s, self.provider.id, reason, self.ctx.now(), escalate=escalate)),
            f"infra {self.provider.id}")
        if escalate and streak:
            self._ask_about_infra(streak[-1], reason)
        where = f"on `{self.provider.id}`"
        if not self.result.get("no_result"):
            set_state_label(self.ctx, number, self._labels(number), label)
            self.gh.create_comment(number, f"The run could not work on this {where} "
                                   f"({self._link()}): {reason}\n\nThat is not this item's fault; "
                                   "it stays queued, for another subscription meanwhile.")
            return
        if self._strike(number, f"a run died: {reason}"):
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

    def _keep_work(self, number: int, kind: str) -> tuple[bool, str]:
        """Keep what a run that did not finish made: a build's on its own branch, a revision's
        on `bot/wip/<pr>` (#317 part 3), which the next revision starts from. Before this a
        revision cut off by a usage cap threw its work away: #143 lost 16 Opus runs so."""
        if kind == "build":
            return self._publish(approved=False)
        return self._publish_wip(number)

    def _publish_wip(self, number: int) -> tuple[bool, str]:
        """Push an unfinished revision's bundle to `bot/wip/<pr>`, never to the pull request's
        branch. The same checks as `_publish`, conflict markers aside (a resolution may stop
        half-way): the head the result names, descending from where the work started, the pull
        request's branch not moved meanwhile, and no forbidden path. `(pushed, problem)`."""
        name = self.result.get("bundle")
        branch = str(self.plan.get("branch") or "")
        if not name or not branch:
            return False, ""
        bundle = self.out_dir / str(name)
        if not bundle.is_file():
            return False, "the result names a bundle the model job did not upload"
        wip = wip_branch(number)
        default = self.cfg.default_branch
        for ref in (default, branch, wip):
            self.repo.run("fetch", "--quiet", "--no-tags", "origin",
                          f"+refs/heads/{ref}:refs/remotes/origin/{ref}", check=ref == default)
        local = f"deliver/{wip}"
        try:
            head = self.repo.fetch_bundle(bundle, branch, local)
        except GitError as exc:
            return False, f"the bundle could not be read: {exc}"
        start = str(self.result.get("start") or "")
        if head != self.result.get("head") or not start or self.repo.run(
                "merge-base", "--is-ancestor", start, head, check=False).returncode != 0:
            return False, "the bundle does not hold the work the result names"
        remote = self.repo.rev(f"origin/{branch}")
        if remote and remote != start:
            return False, f"`{branch}` moved on GitHub while I worked, so the work was not kept"
        merged_main = self.repo.merge_base(f"origin/{default}", local)
        if self.repo.unsanctioned(local, [start, merged_main], self.cfg.forbidden_paths):
            return False, "the work touches paths the bot may not change, so it was not kept"
        if self.cfg.dry_run:
            self.log.append(f"dry run: would push {head} to {wip}")
            return True, ""
        url = f"{self.cfg.server_url}/{self.cfg.repo}.git"
        old = self.repo.rev(f"origin/{wip}")
        try:
            if old and self.repo.run("merge-base", "--is-ancestor", old, head,
                                     check=False).returncode != 0:
                # The run started over from the pull request's head: the old work goes.
                self.repo.push(url, f":refs/heads/{wip}", self.cfg.write_token)
            self.repo.push(url, f"{local}:refs/heads/{wip}", self.cfg.write_token)
        except GitError as exc:
            return False, f"keeping the work on `{wip}` was refused: {exc}"
        self._remember(number, wip={"sha": head, "start": start, "at": iso(self.ctx.now()),
                                    "provider": self.provider.id})
        self.log.append(f"kept the unfinished revision of #{number} on {wip}")
        return True, ""

    def _drop_wip(self, number: int) -> None:
        """A revision delivered, or the pull request closed: its unfinished work is done with."""
        if not self._record(number).get("wip"):
            return
        url = f"{self.cfg.server_url}/{self.cfg.repo}.git"
        try:
            if not self.cfg.dry_run:
                self.repo.push(url, f":refs/heads/{wip_branch(number)}", self.cfg.write_token)
        except GitError:
            pass  # gone already
        self.ctx.store.update(lambda s: state_item(s, number).pop("wip", None),
                              f"wip #{number} done")

    def _progressed(self, number: int, kind: str, before: str) -> bool:
        """Whether a cut-off run's kept work moved on: a commit of its own, not a merge of
        `main`, past `before` (the head the last cut-off run kept, `bot/wip/<pr>`), or past where
        this run started when it did not build on that. A revision that resumes from its wip
        pushes it again every time, so "pushed" alone is no progress."""
        head = str(self.result.get("head") or "")
        if not head:
            return False
        for base in (before, str(self.result.get("start") or "")):
            if not base or self.repo.run("merge-base", "--is-ancestor", base, head,
                                         check=False).returncode != 0:
                continue
            # Not main's: a merge of `main` brings its squash commits in, and they are no merges.
            main = f"origin/{self.cfg.default_branch}"
            ours = ["^" + main] if self.repo.rev(main) else []
            found = self.repo.run("rev-list", "--no-merges", "--count", head, f"^{base}", *ours,
                                  check=False)
            return found.returncode == 0 and (found.stdout or "0").strip() not in ("", "0")
        return False

    def _interrupted(self, number: int, kind: str) -> None:
        interrupt = str(self.result.get("interrupt") or "budget")
        reason = self.result.get("reason")
        kept = self._record(number).get("wip") if kind == "revise" else None
        before = str(kept.get("sha") or "") if isinstance(kept, dict) else ""
        pushed, problem = self._keep_work(number, kind)
        moved = pushed and self._progressed(number, kind, before)
        try:
            calls = int(self.result.get("model_calls") or 0)
        except (TypeError, ValueError):
            calls = 0
        # A cut-off run that made progress (its work moved on) is a step of a long job, not a
        # failure; one that made none counts, whatever cut it off, except a halt, the machine's
        # disk, or a run stopped before any model call (#317 part 3). Before this only the time
        # budget counted, so a usage pause could repeat for ever.
        if interrupt in ("budget", "usage", "died") and calls:
            def count(s: dict[str, Any]) -> None:
                entry = state_item(s, number)
                entry["interruptions"] = 0 if moved else int(entry.get("interruptions", 0)) + 1
            state = self.ctx.store.update(count, f"interrupted #{number}")
            count_now = int(state_item(state, number).get("interruptions", 0))
            if count_now >= self.cfg.max_failures:
                set_state_label(self.ctx, number, self._labels(number), LABEL_BLOCKED)
                self._remember(number, forced=False)
                self.gh.create_comment(number, f"This was cut off {count_now} runs in a row "
                                       f"without its work moving on ({self._link()}; last: "
                                       f"{reason}), so I stopped. It may be too big for one run: "
                                       "split it into smaller issues, or queue it again to keep "
                                       "going.")
                return
        self._requeue_label(number, kind)
        self._remember(number, last_findings=self.result.get("findings") or [])
        where = (f"on `{wip_branch(number)}`, where the next revision starts"
                 if kind == "revise" else "on the branch")
        kept = f" The work so far is {where}." if pushed else ""
        self.gh.create_comment(number, f"Paused: {reason}.{kept} It stays queued and the next "
                               "run picks it up from its notes and its "
                               f"{self._journal_link(number)}."
                               + (f"\n\n{problem}" if problem else ""))

    def _builder_worked(self) -> bool:
        """Some builder session of the run worked: it ended well, or it changed something."""
        cycles = self.result.get("cycles") or []
        if not cycles:
            return True  # a result from before rounds were recorded, or a test's
        return any((c.get("builder") or {}).get("ok") or (c.get("builder") or {}).get("changed")
                   for c in cycles if isinstance(c, dict))

    def _easy_breach(self, number: int, kind: str) -> bool:
        """A weak builder's change on an easy item that breaks the easy rule (`easy.change_breach`:
        too many files or lines, or an off-limits path) is not pushed (#317 part 8): its work is
        kept for a stronger builder (a build's on its own branch, with no pull request; a
        revision's on `bot/wip/<pr>`), the item becomes medium (its label, or a floor under a
        person's label), and it is queued again. True when it did so."""
        if self.build_seat is None or self.build_seat.tier != "weak":
            return False
        if self._difficulty(number) != "easy":
            return False
        counts = self._bundle_counts()
        breach = easy_mod.change_breach(counts, self.cfg.easy) if counts else ""
        if not breach:
            return False
        if kind == "build":
            pushed, _ = self._publish(approved=False)
        else:
            pushed, _ = self._publish_wip(number)
        issue, pr = stepup.pair(self.ctx, number)
        for target in {issue, *([pr] if pr else [])}:
            labels = self._labels(target)
            if rating_source(labels, self._record(target)) == "person":
                self._remember(target, difficulty_floor="medium")
            else:
                self._set_difficulty(target, "medium", labels)
                self._remember(target, difficulty_by={
                    "difficulty": "medium", "provider": "easy rule", "tier": "",
                    "at": iso(self.ctx.now()), "why": breach})
            if target == pr:
                self._remember(target, difficulty="medium")
        self._requeue_label(number, kind)
        kept = ""
        if pushed:
            kept = (f" Its work is kept on `{self.plan.get('branch')}`" if kind == "build"
                    else f" Its work is kept on `{wip_branch(number)}`") + ", for it to go on from."
        self.gh.create_comment(number, f"Not pushed ({self._link()}): {self.build_seat.describe()} "
                               f"built it as `difficulty:easy`, but the change breaks the easy "
                               f"rule ({breach}), so it is `difficulty:medium` now and a medium "
                               f"or stronger model takes it.{kept}")
        return True

    def _bundle_counts(self) -> dict[str, int]:
        """Lines added plus removed per path of the bundle's head against `main`."""
        name = self.result.get("bundle")
        branch = str(self.plan.get("branch") or "")
        if not name or not branch or not (self.out_dir / str(name)).is_file():
            return {}
        default = self.cfg.default_branch
        try:
            self.repo.run("fetch", "--quiet", "--no-tags", "origin",
                          f"+refs/heads/{default}:refs/remotes/origin/{default}")
            local = f"deliver/measure/{branch}"
            self.repo.fetch_bundle(self.out_dir / str(name), branch, local)
            return self.repo.numstat(f"origin/{default}", local)
        except GitError:
            return {}

    def _strike(self, number: int, why: str) -> bool:
        """One failure of the item's own (`stepup.strike`): True when it was its third, and the
        item was stepped up (or blocked at hard) instead of ending the way the caller would."""
        try:
            return stepup.strike(self.ctx, number, why, link=self._link())
        except GitHubError as exc:
            self.log.append(f"could not count a strike on #{number}: {exc}")
            return False

    def _fail(self, number: int, kind: str) -> None:
        """A run that produced nothing usable: requeue it, or block it after too many."""
        if self._strike(number, f"the run failed: {self.result.get('reason') or 'unknown'}"):
            return
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
        if status in ("approved", "built", "not_approved") and not self._builder_worked():
            # #141 was opened by a run whose builder failed with a 401 in 0.0 minutes, from
            # commits earlier runs had left on the branch (#317 part 4).
            self.result["reason"] = ("no builder session in the run worked: "
                                     + str(self.result.get("reason") or "no reason recorded"))
            self._fail(number, "build")
            return
        if status in ("approved", "built") and self._easy_breach(number, "build"):
            return
        approved = status == "approved"
        built = status == "built"  # no model in the run could review it: a review run will
        pushed, problem = self._publish(approved=approved)
        if problem.startswith("the push was refused"):
            self.result.update(status="infra", reason=problem, no_result=True,
                               infra_scope="github")
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
            pull = self._create_pull(title, branch, body, draft=not (approved or built))
        else:
            self.gh.update_pull(int(pull["number"]), title=title, body=body)
            if (approved or built) and pull.get("draft"):
                self._try(lambda: self.gh.mark_ready(pull["node_id"]))
        pr = int(pull.get("number") or 0)
        if pr:
            self.gh.add_labels(pr, [LABEL_PR, *self._carried(number)])
            self._remember(pr, issue=number, feedback_since=started, failures=0, kind="revise",
                           difficulty=self._difficulty(number),
                           self_check_findings=self.result.get("self_check_findings") or [])
        issue_labels = self._labels(number)
        if approved or built:
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
                head = str(self.result.get("head") or "")
                rule = (self._approved(pr, head, merge=not merge_note) if approved
                        else self._await_review(pr, head, queue=not merge_note))
                merge_note = merge_note or rule
            cycles = len(self.result.get("cycles") or [])
            if approved:
                said = (f"Its adversarial reviewer, {self.review_seat.describe()}, approved it on "
                        f"round {cycles} of {self.cfg.max_review_cycles}.")
            else:
                said = self._self_check_said()
            if approved:
                self._unstick(number, pr)
            self.gh.create_comment(number, f"Opened #{pr}, built on {self._built_on()}. "
                                   f"{said} {merge_note}")
            self._remember(number, last_findings=[], question="", failures=0, pr=pr)
        else:
            if self._strike(number, f"its reviewer did not approve it: "
                            f"{self._first_finding() or self.result.get('reason')}"):
                return
            set_state_label(self.ctx, number, issue_labels, LABEL_BLOCKED)
            if pr and LABEL_REVISE not in self._labels(pr):
                set_state_label(self.ctx, pr, self._labels(pr), LABEL_BLOCKED)
            stuck = self._mark_stuck(number, pr)
            self.gh.create_comment(number, f"{self._not_approved_head()}, so #{pr} stays a draft "
                                   f"and will not merge by itself ({self._link()}).\n\n"
                                   f"{self._findings_md()}\n\n{self._why_md()}\n\n{stuck}"
                                   f"Comment `@{self.cfg.bot_login} <guidance>` here or on #{pr} "
                                   "to have me try again with your notes.")
            self._remember(number, last_findings=self.result.get("findings") or [], pr=pr)

    def _not_approved_head(self) -> str:
        """The first words of a comment on a build or revision that was not approved: the real
        round count, and the real reason when it stopped before its last round."""
        rounds = len(self.result.get("cycles") or [])
        limit = self.cfg.max_review_cycles
        if failures.stuck(self.result.get("cycles"), limit):
            return f"After all {limit} rounds the reviewer still had blocking findings"
        reason = str(self.result.get("reason") or "no reason recorded")
        return f"It stopped without an approval after {rounds} of {limit} round(s): {reason}"

    def _why_md(self) -> str:
        return failures.why(self.result.get("cycles"), self.cfg.max_review_cycles,
                            str(self.result.get("reason") or ""))

    def _mark_stuck(self, *numbers: int) -> str:
        """Label a run that used every round `bot:stuck`, for a person to review; the sentence
        that says so, or "" for one that stopped sooner."""
        if not failures.stuck(self.result.get("cycles"), self.cfg.max_review_cycles):
            return ""
        for number in numbers:
            if number:
                self._try(lambda n=number: self.gh.add_labels(n, [LABEL_STUCK]))
        return (f"Labelled `{LABEL_STUCK}`: it failed every round, so a person should read the "
                "above before anyone tries again. ")

    def _unstick(self, *numbers: int) -> None:
        """An approval clears `bot:stuck`."""
        for number in numbers:
            if number and LABEL_STUCK in self._labels(number):
                self._try(lambda n=number: self.gh.remove_label(n, LABEL_STUCK))

    def _self_check_said(self) -> str:
        """What a change built without a reviewer in its run went through."""
        rounds = sum(len(c.get("self_check") or []) for c in self.result.get("cycles") or [])
        open_findings = len(self.result.get("self_check_findings") or [])
        if not rounds:
            return "No model on its subscription may review it."
        if open_findings:
            return (f"It checked itself {rounds} time(s) and ran out with {open_findings} "
                    "finding(s) still open, which its reviewer gets.")
        return f"It checked itself {rounds} time(s), the last one clean; that is not a review."

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
        if status in ("approved", "built") and self._easy_breach(number, "revise"):
            return
        # As the run found it: the source and the clearance `_carry` reads are reset below.
        before = self._record(number)
        started = before.get("started_at")
        if status == "built" and not bot_pr:
            # No review run follows on a person's pull request: unreviewed work is not pushed.
            status = "not_approved"
        if status not in ("approved", "built"):
            self._drop_wip(number)  # nothing of it ships: the next revision starts afresh
            if self._strike(number, f"a revision was not approved: "
                            f"{self._first_finding() or self.result.get('reason')}"):
                return
            set_state_label(self.ctx, number, self._labels(number), LABEL_BLOCKED)
            self._hold_auto_merge()
            stuck = self._mark_stuck(number)
            self.gh.create_comment(number, f"I pushed nothing from this revision. "
                                   f"{self._not_approved_head()} ({self._link()}).\n\n"
                                   f"{self._findings_md()}\n\n{self._why_md()}"
                                   + (f"\n\n{stuck}" if stuck else ""))
            self._remember(number, last_findings=self.result.get("findings") or [],
                           feedback_since=started)
            return
        approved = status == "approved"
        pushed, problem = self._publish(approved=approved)
        if problem:
            set_state_label(self.ctx, number, self._labels(number), LABEL_BLOCKED)
            self._hold_auto_merge()
            self.gh.create_comment(number, f"I could not push the revision ({self._link()}): "
                                   f"{problem}.")
            return
        self._drop_wip(number)
        set_state_label(self.ctx, number, self._labels(number), None)
        if approved:
            self._unstick(number)
        report = str(self.result.get("report") or "")[:REPORT_CHARS]
        rerun = not pushed and bool(self._record(number).get("ci_run_id"))
        # The head a CI fix pushed: CI red on it is a strike (`events.on_ci`, #316).
        fixed = ({"ci_fixed_head": str(self.result.get("head") or "")}
                 if pushed and self._record(number).get("source") == "ci" else {})
        self._remember(number, feedback_since=started, failures=0, source="", last_findings=[],
                       question="",
                       self_check_findings=self.result.get("self_check_findings") or [], **fixed)
        note = ""
        if not pushed and approved:
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
                carry, why_not = self._carry(before, reviewed) if pushed else (None, "")
                if not approved:
                    rule = self._await_review(number, reviewed or now_head, queue=not waiting)
                else:
                    if pull.get("draft") and not waiting:
                        self._try(lambda: self.gh.mark_ready(pull["node_id"]))
                    rule = self._approved(number, reviewed or now_head, merge=not waiting,
                                          carried=carry)
                rule = self._carry_line(before, carry, why_not) + rule
            merge_note = "" if waiting else "\n\n" + rule
        said = (f"{self.review_seat.describe()} reviewed it adversarially and approved it"
                if approved else self._self_check_said())
        self.gh.create_comment(number, f"Revision {'pushed' if pushed else 'done'} "
                               f"({self._link()}) on {self._built_on()}; {said}{note}"
                               f"\n\n{report}{merge_note}")

    # ------------------------------------------------------------------ the review rule

    def _difficulty(self, number: int) -> str:
        """The item's difficulty: its labels, and what its record carries (a bot PR keeps its
        issue's)."""
        return difficulty_of(self._labels(number),
                             str(self._record(number).get("difficulty") or ""))

    def _approvals(self, votes: dict[str, Any]) -> list[tuple[str, str]]:
        """The head's approvals as (family, tier), one per review."""
        return review_rule.approvals(votes, self.cfg.pool.family_tier)

    def _rule_met(self, votes: dict[str, Any], difficulty: str = "medium") -> bool:
        """A commit ships when its approvals meet the review rule for its difficulty
        (`review_rule.met`) and no model's rejection of it stands (a model that rejected it and
        later approved it has withdrawn it)."""
        approvals = set(votes.get("approvals") or [])
        if set(votes.get("rejections") or []) - approvals:
            return False
        if review_rule.carried(votes):
            return True
        return review_rule.met((tier for _, tier in self._approvals(votes)), difficulty)

    def _vote(self, pr: int, head: str, *, approve: bool, builder: bool,
              carried: dict[str, Any] | None = None) -> dict[str, Any]:
        """Record this run's reviewer's verdict on `head`, with its tier: an approval adds one
        review (once per run and family), a rejection withdraws that family's approvals of the
        head. A new
        head starts afresh: its builder is this run's model when this run built it, and unknown
        otherwise. `carried` is the clearance a conflict resolution carries to it (`_carry`)."""
        family = self.review_seat.family if self.review_seat else self.provider.family
        tier = self.review_seat.tier if self.review_seat else "weak"
        run = self.cfg.run_id
        votes: dict[str, Any] = {}

        def change(state: dict[str, Any]) -> None:
            entry = state_item(state, pr)
            current = dict(entry.get("votes") or {})
            if not head or current.get("sha") != head:
                current = {"sha": head, "builder": self.provider.family if builder else "",
                           "approvals": [], "rejections": [], "tiers": {}, "reviews": []}
            reviews = current.get("reviews")
            if not isinstance(reviews, list):
                # A record from before reviews were counted one by one: one per family.
                reviews = [{"family": f, "tier": t}
                           for f, t in review_rule.approvals(current, self.cfg.pool.family_tier)]
            if not approve:
                reviews = [r for r in reviews if r.get("family") != family]
            elif not (run and any(r.get("run") == run and r.get("family") == family
                                  for r in reviews)):
                # Once per run: a deliver job run again must not count its review twice.
                review = {"family": family, "tier": tier, **({"run": run} if run else {})}
                reviews = [*reviews, review]
            key, other = ("approvals", "rejections") if approve else ("rejections", "approvals")
            current[key] = list(dict.fromkeys([*current.get(key, []), family]))
            current[other] = [f for f in current.get(other, []) if f != family]
            current["tiers"] = {**(current.get("tiers") or {}), family: tier}
            current["reviews"] = reviews
            if carried:
                current["carried"] = carried
            entry["votes"] = current
            votes.update(current)

        self.ctx.store.update(change, f"votes #{pr}")
        return votes

    def _approved(self, pr: int, head: str, *, merge: bool = True, vote: bool = True,
                  builder: bool = True, carried: dict[str, Any] | None = None) -> str:
        """Record this run's reviewer's approval of `head` on the PR, then turn on auto-merge
        when the rule is met (recording `head` as cleared); otherwise turn it off and queue a
        review run. Returns the line for the comment."""
        if vote:
            votes = (self._vote(pr, head, approve=True, builder=builder, carried=carried)
                     if self.review_seat is not None
                     else self._vote_reset(pr, head, self.provider.family if builder else ""))
        else:
            votes = self._vote_reset(pr, head)
        difficulty = self._difficulty(pr)
        met = self._rule_met(votes, difficulty)
        if met:
            self._clear(pr, head, votes)
        if not merge:
            return ""
        labels = self._labels(pr)
        if LABEL_PR not in labels:
            return ""  # never auto-merge a pull request the bot did not open
        if met:
            return self._auto_merge(self.gh.get_pull(pr), expected_head=head)
        return self._wait_for_review(pr, votes, labels, difficulty)

    def _clear(self, pr: int, head: str, votes: dict[str, Any]) -> None:
        """Record `head` as cleared: its approvals met the review rule. A conflict with `main`
        later needs only the resolving run's own review (`_carry`). A clearance carried across
        a resolution keeps who cleared the change, and adds who reviewed the resolution."""
        who = review_rule.who(self._approvals(votes))
        entry: dict[str, Any] = {"sha": head, "at": iso(self.ctx.now()), "by": who}
        # The item's strikes end once this head is green in CI too (#316): the sweep sees that
        # (`sweep._green_heads`), and a merge does (`events.on_pull_closed`).
        carried = votes.get("carried")
        if isinstance(carried, dict):
            entry.update(by=str(carried.get("by") or who), resolved_by=who,
                         carried_from=str(carried.get("from") or ""))
        self._remember(pr, cleared=entry)

    def _carry(self, record: dict[str, Any], head: str) -> tuple[dict[str, Any] | None, str]:
        """Whether a revision carries its change's clearance to `head` (`review_rule`): it was
        queued because `main` left the change with conflicts, it started from the commit the
        rule cleared, a medium or strong model reviewed it in the run, and it changed nothing but
        the files the merge left conflicted. That last is worked out here from git, never from
        the model job: `head` against the merge git makes by itself of the cleared commit and the
        `main` that `head` took in. Returns the carry, or None and why not (empty when the
        revision was not a conflict on a cleared change)."""
        clear = cleared(record)
        if record.get("source") != "conflict" or not clear:
            return None, ""
        start = str(self.result.get("start") or "")
        if clear["sha"] != start:
            return None, "the branch had moved on from the commit they cleared"
        seat = self.review_seat
        if seat is None or not providers_mod.tier_at_least(seat.tier, "medium"):
            return None, "no medium or strong model reviewed the resolution in this run"
        try:
            main = self.repo.merge_base(f"origin/{self.cfg.default_branch}", head)
            proc = self.repo.run("merge-tree", "--write-tree", "--name-only", "--no-messages",
                                 start, main, check=False)
            lines = [line.strip() for line in proc.stdout.splitlines() if line.strip()]
            if proc.returncode not in (0, 1) or not lines:
                return None, "git could not redo the merge to compare against"
            tree, conflicts = lines[0], lines[1:]
            beyond = [p for p in self.repo.names_between(tree, head) if p not in conflicts]
        except GitError:
            return None, "git could not redo the merge to compare against"
        if beyond:
            shown = ", ".join(f"`{p}`" for p in beyond[:8]) + (", …" if len(beyond) > 8 else "")
            return None, f"it changed more than the conflicts ({shown})"
        return {"from": start, "main": main, "conflicts": conflicts,
                "by": str(clear.get("by") or "")}, ""

    def _carry_line(self, record: dict[str, Any], carry: dict[str, Any] | None,
                    why_not: str) -> str:
        """What a conflict revision of a cleared change did with the clearance, for the comment
        ("" for any other revision)."""
        clear = cleared(record)
        if not clear or not (carry or why_not):
            return ""
        by = f" ({clear['by']})" if clear.get("by") else ""
        said = f"The reviews had cleared this change at `{str(clear['sha'])[:12]}`{by}"
        if carry is None:
            return f"{said}, but that does not carry over: {why_not}. "
        files = carry.get("conflicts") or []
        resolved = ("resolved the conflicts in " + ", ".join(f"`{p}`" for p in files[:8])
                    + (", …" if len(files) > 8 else "")) if files else "merged it cleanly"
        return (f"{said}. This revision only merged `main` and {resolved}, and its own reviewer "
                "approved that, so the clearance carries over and no review run is needed. ")

    def _await_review(self, pr: int, head: str, *, queue: bool = True) -> str:
        """A change no model in its run could review: its commit has no vote yet, and it waits
        for a review run."""
        votes = self._vote_reset(pr, head, self.provider.family)
        if not queue:
            self._hold_auto_merge(pr)
            return ""
        labels = self._labels(pr)
        if LABEL_PR not in labels:
            return "A person reviews it."
        return self._wait_for_review(pr, votes, labels, self._difficulty(pr))

    def _wait_for_review(self, pr: int, votes: dict[str, Any], labels: set[str],
                         difficulty: str) -> str:
        self._hold_auto_merge(pr)
        set_state_label(self.ctx, pr, labels, LABEL_CROSS)
        self._remember(pr, queued_at=iso(self.ctx.now()))
        approvals = self._approvals(votes)
        have = (f"{review_rule.who(approvals)} approved it" if approvals
                else "No model has reviewed it yet")
        need = review_rule.missing((tier for _, tier in approvals), difficulty)
        line = f"{have}, so it waits for {need} (`{LABEL_CROSS}`) before auto-merge turns on."
        if not self._reviewer_set_up(votes, difficulty):
            line += (" No subscription that could give that review is set up, so it waits for "
                     "you to merge it, or for one to be set up.")
        return line

    def _vote_reset(self, pr: int, head: str, builder: str = "") -> dict[str, Any]:
        """A head no model reviewed (someone pushed during the run, or no model here may
        review): its votes start empty."""
        votes = {"sha": head, "builder": builder, "approvals": [], "rejections": [], "tiers": {},
                 "reviews": []}
        self._remember(pr, votes=votes)
        return votes

    def _reviewer_set_up(self, votes: dict[str, Any], difficulty: str) -> bool:
        """Whether the set-up subscriptions could still give the reviews the head needs
        (`review_rule.reachable`)."""
        secrets = self.cfg.secrets
        available = {seat.tier for provider in self.cfg.pool.ordered()
                     if provider.enabled and "review" in provider.roles
                     and (provider.login == "machine" or secrets.has(provider.secret) is not False)
                     for seat in self.cfg.pool.own_seats(provider)}
        return review_rule.reachable([tier for _, tier in self._approvals(votes)], available,
                                     difficulty)

    def _carried(self, issue: int) -> list[str]:
        """The issue's labels its pull request starts with, approved or a draft: its difficulty,
        so the same models revise and review it, and its priority tier (#90)."""
        return sorted(name for name in self._labels(issue)
                      if name.lower() in {*DIFFICULTY_LABELS, *PRIORITY_TIERS})

    def _second_review(self, number: int, status: str) -> None:
        """Act on a review run's verdict on a bot pull request."""
        record = self._record(number)
        if status not in ("infra", "interrupted"):
            # A review a person asked for is answered: the next one takes any tier again.
            self._remember(number, review_floor="", review_notes="")
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
        if self._strike(number, f"{who} rejected it: {self._first_finding() or 'see its findings'}"):
            return
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
        """The review rule approved the head but auto-merge cannot take it: label it `ready for
        merge` and ask the operator to merge it."""
        number = int(pull.get("number") or 0)
        if number:
            self.gh.add_labels(number, [LABEL_NEEDS_REVIEW, LABEL_READY])
            self._try(lambda: self.gh.request_reviewers(number, [self.cfg.operator]))
        return (f"Auto-merge is off because {why}. The reviews approved it, so it is labelled "
                f"`{LABEL_READY}`: @{self.cfg.operator}, it waits for you to merge it.")

    def _auto_merge(self, pull: dict[str, Any], expected_head: str = "") -> str:
        """Turn on auto-merge only when nothing needs a person and CI must pass first, pinned to
        `expected_head` (the approved commit) when given."""
        if not self.cfg.auto_merge:
            if pull.get("number"):
                self.gh.add_labels(int(pull["number"]), [LABEL_READY])
            return (f"Auto-merge is off in `.harness/config.json`, so it is labelled "
                    f"`{LABEL_READY}`; a person merges it.")
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
        headline = self._merge_title(pull)
        error = self._try(lambda: self.gh.enable_auto_merge(pull["node_id"], self.cfg.merge_method,
                                                            expected_head, headline))
        if not error:
            return "Auto-merge is on: it merges when every required check passes."
        if "clean status" in error.lower():
            # Every required check has already passed, so GitHub will not wait; merge it now.
            merged = self._try(lambda: self.gh.merge_pull(int(pull["number"]), self.cfg.merge_method,
                                                          headline))
            if not merged:
                return "Every required check had already passed, so I merged it."
        return self._hand_to_a_person(pull, f"GitHub refused it ({error})")

    def _merge_title(self, pull: dict[str, Any]) -> str:
        """The squash commit's title: the pull request's, as GitHub writes it for a pull request of
        several commits. Without it a one-commit pull request lands on `main` under the bot's own
        commit message ("bot: build pass 1 for #85")."""
        title = str(pull.get("title") or "").strip()
        if self.cfg.merge_method != "squash" or not title:
            return ""
        return f"{title} (#{pull.get('number')})"

    def _rerun_ci(self, number: int) -> None:
        run_id = self._record(number).get("ci_run_id")
        if run_id:
            self._try(lambda: self.ctx.act.rerun_failed_jobs(run_id))

    def _first_finding(self) -> str:
        """The first blocking finding's claim, cut short, for a strike's reason."""
        findings = self.result.get("findings") or []
        if not findings or not isinstance(findings[0], dict):
            return ""
        where = str(findings[0].get("where") or "")
        claim = " ".join(str(findings[0].get("claim") or "").split())[:140]
        return f"`{where}`: {claim}" if where else claim

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
            parts.append(f"### Not approved\n\n{self._not_approved_head()}, so this is a draft "
                         "and will not merge by itself.")
            parts.append("")
            parts.append(self._findings_md())
        rows = ["| Round | Builder | Checks | Review |", "|---|---|---|---|"]
        for c in cycles:
            builder = c.get("builder") or {}
            checks = c.get("gates") or []
            red = [g["name"] for g in checks if not g.get("ok") and not g.get("pre_existing")
                   and not g.get("inconclusive")]
            verdict = (c.get("review") or {}).get("verdict", "-")
            blocking = len([f for f in (c.get("review") or {}).get("findings") or []
                            if f.get("severity") == "blocking"])
            rows.append(f"| {c.get('n')} | {builder.get('role')}, {builder.get('minutes')} min | "
                        f"{'red: ' + ', '.join(red) if red else 'green'} | {verdict}"
                        f"{f' ({blocking} blocking)' if blocking else ''} |")
        results = [gates_mod.GateResult(**{k: g.get(k) for k in (
            "name", "run", "ok", "exit_code", "seconds", "tail", "pre_existing", "skipped",
            "inconclusive")})
            for g in self.result.get("gates") or []]
        parts += ["", "<details><summary>Rounds and checks</summary>", "", *rows, "",
                  gates_mod.table(results), "", "</details>", ""]
        run = f" ([run]({self.cfg.run_url}))" if self.cfg.run_url else ""
        parts.append(f"---\nBuilt overnight by @{self.cfg.bot_login}{run}; every run on it, "
                     f"step by step, is in its {self._journal_link(number)}. Comment "
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
                      f"Add the `{LABEL_APPROVED}` label to have it built: two minutes later "
                      "triage titles, labels and queues it as it does `method:use-bot`, and the "
                      "bot rates its difficulty when it plans it. Close it to say no.")
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
