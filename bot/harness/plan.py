"""The first job of a night run: decide what this run does and which subscription does it.

It spends nothing. In order: the two kill switches, housekeeping (items a dead run left
`bot:working`, bot PRs that conflict with `main`), the lanes (at most `max_parallel` runs hold an
item at once, and a subscription holds one at a time), then one item and the subscription that
takes it (`pairs`), or a suggestion survey when nothing is queued and one is due. Having claimed
an item, it starts another run when a lane and more work are still free, so the lanes fill up.

Which subscription takes an item (`pairs`): items go in order of urgency (forced, then the
priority tier, `difficult`, second reviews, revisions, the oldest builds), and each goes to the
first subscription in `priority` that is free, available (`providers.availability`: switched on,
its secret set, inside its hours unless the item is forced, under its limits) and allowed it: a
`difficult` item only to one marked `difficult` (Opus), a second review only to a model family
that has not approved the change yet, a `shitter` item only to a low-tier model. An item
labelled `human` is never queued (`candidates`). `claude-1` is shared with its owner, so an
unforced run uses it only after the gate saw it quiet (`quiet_ok`).
"""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import timedelta
from itertools import islice
from typing import Any, Iterator

from harness import asks, threads
from harness import providers as providers_mod
from harness.clock import iso, parse_iso
from harness.config import (LABEL_BLOCKED, LABEL_BUILD, LABEL_CROSS, LABEL_PR, LABEL_PR_OPEN,
                            LABEL_REVISE, LABEL_SUGGESTION, LABEL_WORKING, NIGHT_WORKFLOW,
                            STATE_BRANCH)
from harness.context import Context
from harness.errors import GitHubError
from harness.prompts import data
from harness.providers import Provider
from harness.queue import (KIND_ORDER, PRIORITY_NAMES, Candidate, branch_for_issue, candidates,
                           label_names, open_pull_for_branch, set_state_label)
from harness.state import item as state_item

MODES = ("auto", "build", "revise", "review", "suggest")
#: A run asked for and never taken up is dropped after this long.
RUN_REQUEST_TTL = timedelta(hours=12)
CI_LOG_JOBS = 4
#: A run in one of these states is still going.
LIVE = ("queued", "in_progress", "waiting", "pending", "requested")
#: The role a provider must allow for each kind of item.
ROLE_OF = {"build": "build", "revise": "revise", "review": "review"}
#: `quiet_ok` that lets every provider through, for questions that start nothing.
ANY_QUIET = "*"
#: The gate's step that waits for the shared subscription to be quiet (`bot-night.yml`).
QUIET_STEP = "Wait until the subscription is quiet"


def run_link(cfg: Any) -> str:
    """` ([run](url))` for a run in Actions, and nothing for one started by hand."""
    return f" ([run]({cfg.run_url}))" if cfg.run_url else ""


def nothing(reason: str) -> dict[str, Any]:
    return {"action": "none", "reason": reason}


def run_status(ctx: Context, run_id: Any) -> str:
    """`alive`, `dead`, or `unknown` when GitHub could not say. Runs now overlap, so a run that
    cannot be read is treated as alive: its lane stays held and its item is not requeued."""
    if not run_id:
        return "dead"
    try:
        return "alive" if ctx.gh.get_run(run_id).get("status") in LIVE else "dead"
    except GitHubError as exc:
        return "dead" if exc.status == 404 else "unknown"


def run_alive(ctx: Context, run_id: Any) -> bool:
    """True unless the run is known to have ended."""
    return run_status(ctx, run_id) != "dead"


def vault_path(provider_id: str) -> str:
    """Where a provider's refreshed login is kept, encrypted, on the state branch (`vault.py`)."""
    return f"vault/{provider_id}.enc"


def stops(ctx: Context, state: dict[str, Any], force: bool) -> str | None:
    """Why no run may start now, whatever is queued, or None. `make` and `peek` share it."""
    cfg = ctx.cfg
    if ctx.repo_halted():
        return "halted by .harness/HALT on main"
    if state.get("halted"):
        return "halted by /harness halt"
    if cfg.secrets.known and not any(p.enabled and (p.login == "machine" or cfg.secrets.has(p.secret))
                                     for p in cfg.pool.ordered()):
        return "no subscription has its secret set, so no model can run"
    # A run that could not work backs off its own subscription only (providers.INFRA_BACKOFF).
    return None


# ---------------------------------------------------------------------- lanes and matching


@dataclass
class Lanes:
    """The runs that hold work now: item number (0 for a suggestion survey) -> provider id."""

    limit: int
    held: dict[int, str] = field(default_factory=dict)

    @property
    def busy(self) -> set[str]:
        return set(self.held.values())

    @property
    def free(self) -> int:
        return max(0, self.limit - len(self.held))

    def describe(self) -> str:
        return ", ".join(f"#{n} with `{p}`" if n else f"a survey with `{p}`"
                         for n, p in sorted(self.held.items())) or "none"


def working_threads(ctx: Context) -> list[dict[str, Any]]:
    """Every thread labelled `bot:working`, closed ones too: a run goes on after someone closes
    its issue, and holds its lane until it ends (deliver then drops its work)."""
    return ctx.gh.list_issues(labels=LABEL_WORKING, state="all")


def read_lanes(ctx: Context, state: dict[str, Any]) -> Lanes:
    """The lanes held by runs that are still going. A run that ended without delivering holds
    nothing: housekeeping requeues its item."""
    lanes = Lanes(ctx.cfg.pool.max_parallel)
    for thread in working_threads(ctx):
        number = int(thread["number"])
        record = state["items"].get(str(number), {})
        if run_alive(ctx, record.get("run_id")):
            lanes.held[number] = str(record.get("provider") or providers_mod.LEGACY_PROVIDER)
    survey = state.get("suggest") or {}
    if survey.get("provider") and run_alive(ctx, survey.get("run_id")):
        lanes.held[0] = str(survey["provider"])
    return lanes


def _usable(ctx: Context, state: dict[str, Any], provider: Provider, lanes: Lanes, role: str,
            *, forced: bool, quiet_ok: str) -> bool:
    cfg = ctx.cfg
    if provider.id in lanes.busy or role not in provider.roles:
        return False
    if (provider.quiet_check and cfg.quiet.enabled and not forced
            and quiet_ok not in (ANY_QUIET, provider.id)):
        return False
    return providers_mod.availability(provider, state, ctx.now(), cfg.timezone, cfg.secrets,
                                      forced=forced) is None


def pairs(ctx: Context, state: dict[str, Any], queue: list[Candidate], lanes: Lanes, *,
          force: bool, quiet_ok: str,
          skipped: list[str] | None = None) -> Iterator[tuple[Candidate, Provider]]:
    """Every queued item that a subscription can take now, with the one that takes it, most
    urgent first. A free subscription that passes over a `shitter` item because its model is
    high-tier says so in `skipped`, when the caller keeps one."""
    order = sorted(queue, key=lambda c: (not (force or c.forced), c.priority, not c.difficult,
                                         KIND_ORDER[c.kind], c.queued_at, c.number))
    for candidate in order:
        forced = force or candidate.forced
        for provider in ctx.cfg.pool.ordered():
            if candidate.difficult and not provider.difficult:
                continue
            if candidate.kind == "review" and provider.family == candidate.builder:
                continue
            if not _usable(ctx, state, provider, lanes, ROLE_OF[candidate.kind], forced=forced,
                           quiet_ok=quiet_ok):
                continue
            tier = providers_mod.model_tier(provider.model)
            if candidate.low_tier_only and tier == providers_mod.HIGH_TIER:
                if skipped is not None:
                    skipped.append(f"#{candidate.number} skipped on `{provider.id}`: labelled "
                                   f"`shitter`, and {provider.model} is {tier}-tier")
                continue
            yield candidate, provider
            break


def survey_provider(ctx: Context, state: dict[str, Any], lanes: Lanes, *, force: bool,
                    quiet_ok: str) -> Provider | None:
    for provider in ctx.cfg.pool.ordered():
        if _usable(ctx, state, provider, lanes, "suggest", forced=force, quiet_ok=quiet_ok):
            return provider
    return None


def why_none(ctx: Context, state: dict[str, Any], lanes: Lanes) -> str:
    """Why each subscription cannot take work now, for the run's summary and `status`."""
    cfg = ctx.cfg
    parts = []
    for provider in cfg.pool.ordered():
        if provider.id in lanes.busy:
            parts.append(f"`{provider.id}` is busy")
            continue
        reason = providers_mod.availability(provider, state, ctx.now(), cfg.timezone, cfg.secrets)
        if reason is None and provider.quiet_check and cfg.quiet.enabled:
            reason = "waits for its owner to be quiet"
        parts.append(f"`{provider.id}` {reason or 'is free'}")
    return "; ".join(parts)


def _queue(ctx: Context, state: dict[str, Any], item: int | None, mode: str,
           skipped: list[str] | None = None) -> list[Candidate]:
    queue = candidates(ctx, state, skipped)
    if item is not None:
        queue = [c for c in queue if c.number == int(item)]
    if mode in ("build", "revise", "review"):
        queue = [c for c in queue if c.kind == mode]
    return queue


def quiet_waiting(ctx: Context) -> bool:
    """True when another `bot-night` run is waiting for the shared subscription to be quiet, so
    this one should not wait too (each waiting gate reads the usage every ten minutes)."""
    try:
        for run in ctx.gh.list_runs(NIGHT_WORKFLOW, status="in_progress", limit=10):
            if str(run.get("id")) == ctx.cfg.run_id:
                continue
            for job in ctx.gh.list_jobs(run.get("id")):
                for step in job.get("steps") or []:
                    if step.get("name") == QUIET_STEP and step.get("status") == "in_progress":
                        return True
    except GitHubError:
        return False
    return False


@dataclass
class Peek:
    """What the gate learns: whether a run would find work, and whether it must first wait for
    the shared subscription to be quiet (`quiet_provider`), with `fallback` when other work
    could go ahead without it."""

    work: bool
    reason: str
    forced: bool = False
    provider: str = ""
    quiet_provider: str = ""
    quiet_secret: str = ""
    fallback: bool = False
    held: int = 0
    #: Queued items passed over because of `human` or `shitter` (#96), one line each.
    skipped: list[str] = field(default_factory=list)


def peek(ctx: Context, *, force: bool = False, item: int | None = None,
         mode: str = "auto") -> Peek:
    """Whether a run would find work, why, and with which subscription. Changes nothing.

    The gate asks this before it spends anything on reading the subscription's usage."""
    skipped: list[str] = []
    look = _peek(ctx, force, item, mode, skipped)
    look.skipped = skipped
    return look


def _peek(ctx: Context, force: bool, item: int | None, mode: str, skipped: list[str]) -> Peek:
    if mode not in MODES:
        return Peek(False, f"unknown mode {mode!r}")
    state = ctx.store.load()
    asked = run_request(ctx, state)
    if asked is not None:
        force = True
        item = item if item is not None else asked.get("item")
    stop = stops(ctx, state, force)
    if stop:
        return Peek(False, stop)
    tidy = housekeeping_due(ctx, state)
    lanes = read_lanes(ctx, state)
    if lanes.free <= 0:
        if tidy:
            return Peek(True, tidy, force, held=len(lanes.held))
        return Peek(False, f"every lane is busy ({lanes.describe()})", held=len(lanes.held))
    queue = _queue(ctx, state, item, mode, skipped)
    found: tuple[Candidate | None, Provider, bool] | None = None
    if mode != "suggest":
        top = next(pairs(ctx, state, queue, lanes, force=force, quiet_ok=ANY_QUIET,
                         skipped=skipped), None)
        if top is not None:
            found = (top[0], top[1], force or top[0].forced)
    # A survey only when nothing is queued (or one was asked for), as before: queued work that
    # must wait for its subscription does not turn the others into surveyors.
    if found is None and item is None and (mode == "suggest" or (mode == "auto" and not queue)) \
            and suggestions_due(ctx, state, force=mode == "suggest"):
        provider = survey_provider(ctx, state, lanes, force=force, quiet_ok=ANY_QUIET)
        if provider is not None:
            found = (None, provider, force)
    if found is None:
        if tidy:
            return Peek(True, tidy, force, held=len(lanes.held))
        if item is not None and mode != "suggest":
            return Peek(False, f"#{item} is not queued, or no subscription can take it now: "
                        f"{why_none(ctx, state, lanes)}", held=len(lanes.held))
        if queue:
            return Peek(False, f"no subscription can take the queue now: "
                        f"{why_none(ctx, state, lanes)}", held=len(lanes.held))
        return Peek(False, "nothing is queued", held=len(lanes.held))
    candidate, provider, forced = found
    what = (f"#{candidate.number} is queued to {candidate.kind}" if candidate
            else "a suggestion survey is due")
    reason = f"{what}, for `{provider.id}`"
    if not (provider.quiet_check and ctx.cfg.quiet.enabled and not forced):
        return Peek(True, reason, forced, provider.id, held=len(lanes.held))
    # The best subscription for it is the shared one: wait for quiet, unless another run is
    # already waiting, and say whether other work could go ahead without it.
    other = next(pairs(ctx, state, queue, lanes, force=force, quiet_ok=""), None) if (
        mode != "suggest") else None
    fallback = other is not None
    if other is not None and other[1].family == provider.family:
        # Another account of the same model can take work now: no reason to wait for this one.
        return Peek(True, f"#{other[0].number} is queued to {other[0].kind}, for "
                    f"`{other[1].id}`", force or other[0].forced, other[1].id,
                    held=len(lanes.held))
    if quiet_waiting(ctx):
        if fallback:
            return Peek(True, f"#{other[0].number} is queued to {other[0].kind}, for "
                        f"`{other[1].id}` (another run waits for `{provider.id}`)", forced,
                        other[1].id, held=len(lanes.held))
        return Peek(False, f"another run already waits for `{provider.id}` to be quiet",
                    held=len(lanes.held))
    return Peek(True, reason, forced, provider.id, quiet_provider=provider.id,
                quiet_secret=provider.secret, fallback=fallback, held=len(lanes.held))


def run_request(ctx: Context, state: dict[str, Any]) -> dict[str, Any] | None:
    """A `/harness run` (or `start --force`) not yet taken up, if one is fresh enough."""
    asked = state.get("run_requested")
    at = parse_iso((asked or {}).get("at"))
    if not isinstance(asked, dict) or at is None or ctx.now() - at > RUN_REQUEST_TTL:
        return None
    return asked


def housekeeping_due(ctx: Context, state: dict[str, Any]) -> str | None:
    """What `housekeeping` would requeue, read without changing anything, or None."""
    for thread in working_threads(ctx):
        run_id = state["items"].get(str(thread["number"]), {}).get("run_id")
        if run_status(ctx, run_id) == "dead":
            return f"#{thread['number']} was left working by a run that ended"
    for thread in ctx.gh.list_issues(labels=LABEL_PR):
        names = label_names(thread)
        if "pull_request" not in thread or names & {LABEL_REVISE, LABEL_WORKING, LABEL_BLOCKED}:
            continue
        record = state["items"].get(str(thread["number"]), {})
        if record.get("stop_requested"):
            continue
        if ctx.gh.get_pull(int(thread["number"])).get("mergeable_state") == "dirty":
            return f"#{thread['number']} conflicts with main"
    return None


def make(ctx: Context, *, force: bool = False, item: int | None = None, mode: str = "auto",
         quiet_ok: str | None = None) -> dict:
    """Decide what this run does and with which subscription. A subscription outside its hours
    works only on forced work: the `force` of this dispatch, or an item an operator queued with
    `--force`, which outlives the run it started. `quiet_ok` is the gate's verdict: the shared
    subscription it saw quiet, or "" for none; the workflow always passes one, and None (a run
    by hand) restricts nothing."""
    cfg = ctx.cfg
    quiet = ANY_QUIET if quiet_ok is None else quiet_ok
    if mode not in MODES:
        return nothing(f"unknown mode {mode!r}")
    state = ctx.store.load()
    asked = run_request(ctx, state)
    if asked is not None:
        force = True
        item = item if item is not None else asked.get("item")
    stop = stops(ctx, state, force)
    if stop:
        return nothing(stop)
    notes = housekeeping(ctx, state)

    def taken(planned: dict[str, Any]) -> dict[str, Any]:
        # A `/harness run` is taken up by the run that claims something for it, never by one
        # that found every lane busy: one request, one run.
        if asked is not None:
            ctx.store.update(lambda s: s.update(run_requested=None), "run request taken")
        return planned
    state = ctx.store.load()
    lanes = read_lanes(ctx, state)
    if lanes.free <= 0:
        return {**nothing(f"every lane is busy ({lanes.describe()})"), "housekeeping": notes}
    skipped: list[str] = []
    queue = _queue(ctx, state, item, mode, skipped)
    if mode != "suggest":
        for candidate, provider in pairs(ctx, state, queue, lanes, force=force, quiet_ok=quiet,
                                         skipped=skipped):
            planned = claim(ctx, candidate, provider)
            if planned is not None:
                planned["housekeeping"] = notes
                planned["forced"] = bool(force or candidate.forced)
                planned["priority"] = PRIORITY_NAMES[candidate.priority]
                planned["skipped"] = skipped
                fill_lanes(ctx, lanes, candidate, provider, planned)
                return taken(planned)
    if item is not None and mode != "suggest":
        return {**nothing(f"#{item} is not queued (or has failed {cfg.max_failures} times), or "
                          f"no subscription can take it now: {why_none(ctx, state, lanes)}"),
                "skipped": skipped}
    if mode == "suggest" or (mode == "auto" and not queue):
        planned = suggestion_plan(ctx, lanes, force=mode == "suggest", quiet_ok=quiet)
        if planned is not None:
            planned["skipped"] = skipped
            return taken(planned)
    reason = ("nothing is queued" if not queue else
              f"no subscription can take the queue now: {why_none(ctx, state, lanes)}")
    return {**nothing(reason), "housekeeping": notes, "skipped": skipped}


def fill_lanes(ctx: Context, lanes: Lanes, taken: Candidate, provider: Provider,
               planned: dict[str, Any]) -> None:
    """Start one more run when, with this item claimed, a lane and more work are still free.
    That run does the same, so the lanes fill one run at a time."""
    after = Lanes(lanes.limit, {**lanes.held, taken.number: provider.id})
    if after.free <= 0:
        return
    state = ctx.store.load()
    rest = [c for c in candidates(ctx, state) if c.number != taken.number]
    if next(pairs(ctx, state, rest, after, force=False, quiet_ok=ANY_QUIET), None) is None:
        return
    try:
        ctx.dispatch()
        planned["filled"] = "started another run for the next free lane"
    except GitHubError as exc:
        planned["filled"] = f"could not start another run: {exc}"


def housekeeping(ctx: Context, state: dict[str, Any]) -> list[str]:
    """Requeue items a dead run left working; queue a revision for conflicted bot PRs."""
    notes: list[str] = []
    for thread in working_threads(ctx):
        number = int(thread["number"])
        record = state["items"].get(str(number), {})
        run_id = str(record.get("run_id") or "")
        if run_id and run_id == ctx.cfg.run_id:
            continue
        if run_status(ctx, run_id) != "dead":
            continue
        if thread.get("state") != "open":  # closed meanwhile: nothing to requeue
            set_state_label(ctx, number, label_names(thread), None)
            notes.append(f"#{number} was closed; its dead run's label is gone")
            continue
        if record.get("kind") == "review":
            wanted = LABEL_CROSS
        else:
            wanted = LABEL_REVISE if "pull_request" in thread else LABEL_BUILD
        set_state_label(ctx, number, label_names(thread), wanted)
        ctx.store.update(lambda s, n=number: asks.give_back(state_item(s, n)), f"asks #{number}")
        ctx.gh.create_comment(number, "The run that was working on this ended without "
                              "finishing; it is back in the queue.")
        notes.append(f"requeued #{number} from a dead run")
    survey = state.get("suggest") or {}
    if survey.get("provider") and run_status(ctx, survey.get("run_id")) == "dead":
        ctx.store.update(lambda s: s["suggest"].update(provider=None, run_id=None), "survey ended")
    for thread in ctx.gh.list_issues(labels=LABEL_PR):
        if "pull_request" not in thread:
            continue
        names = label_names(thread)
        if names & {LABEL_REVISE, LABEL_WORKING, LABEL_BLOCKED}:
            continue
        pull = ctx.gh.get_pull(int(thread["number"]))
        record = state["items"].get(str(pull["number"]), {})
        if pull.get("mergeable_state") == "dirty" and not record.get("stop_requested"):
            number = int(pull["number"])
            set_state_label(ctx, number, names, LABEL_REVISE)
            if pull.get("auto_merge") and pull.get("node_id"):
                # The revision may come from another model; its approval decides afresh.
                try:
                    ctx.gh.disable_auto_merge(pull["node_id"])
                except GitHubError:
                    pass
            ctx.store.update(lambda s, n=number: state_item(s, n).update(
                kind="revise", source="conflict", queued_at=iso(ctx.now()), failures=0,
                stop_requested=False), f"conflict #{number}")
            notes.append(f"queued #{number}: it conflicts with main")
    for thread in ctx.gh.list_issues(labels=LABEL_PR_OPEN):
        number = int(thread["number"])
        if open_pull_for_branch(ctx, branch_for_issue(number)) is None:
            ctx.gh.remove_label(number, LABEL_PR_OPEN)
            notes.append(f"#{number} has no open bot PR any more")
    return notes


def _provider_fields(ctx: Context, provider: Provider) -> dict[str, Any]:
    """What the model job needs to know about the subscription it runs on. The secret's value
    is never here: the workflow hands the model job only the secret named."""
    vault, _ = ctx.gh.get_file(vault_path(provider.id), STATE_BRANCH)
    return {"provider": provider.id, "cli": provider.cli, "secret": provider.secret,
            "family": provider.family, "shared": provider.quiet_check, "vault": vault or "",
            "login": provider.login, "runs_on": provider.runs_on}


def claim(ctx: Context, candidate: Candidate, provider: Provider) -> dict[str, Any] | None:
    """Build the plan for one queued thread on one subscription, and mark it `bot:working`."""
    cfg = ctx.cfg
    number, kind = candidate.number, candidate.kind
    thread = ctx.gh.get_issue(number)
    names = label_names(thread)
    if thread.get("state") != "open":
        set_state_label(ctx, number, names, None)
        return None
    state = ctx.store.load()
    record = state["items"].get(str(number), {})
    who = provider.describe()
    if kind == "build":
        planned: dict[str, Any] = {
            "action": "build",
            "number": number,
            "title": thread.get("title", ""),
            "branch": branch_for_issue(number),
            "thread": threads.issue_thread(ctx.gh, ctx.trust, number, cfg.bot_login),
            "previous_findings": record.get("last_findings") or [],
            "previous_question": record.get("question") or "",
        }
        message = (f"Starting work on this now{run_link(cfg)}, on {who}. I build it, run the "
                   f"repository's checks, and have an adversarial reviewer read the change, up to "
                   f"{cfg.max_review_cycles} rounds. Only an approved change becomes a pull request.")
    else:
        pull = ctx.gh.get_pull(number)
        head = pull.get("head") or {}
        if (head.get("repo") or {}).get("full_name") != cfg.repo:
            set_state_label(ctx, number, names, None)
            ctx.gh.create_comment(number, "This pull request comes from a fork, so I cannot push "
                                  "to it. Taking it out of the queue.")
            return None
        issue_number = threads.linked_issue(pull)
        issue_text = ""
        if issue_number:
            try:
                issue_text = threads.issue_thread(ctx.gh, ctx.trust, issue_number, cfg.bot_login)
            except GitHubError:
                issue_number = None
        pull_text = threads.pull_text(pull)
        if kind == "review":
            builder = candidate.builder or "an unknown model"
            planned = {
                "action": "review",
                "number": number,
                "title": pull.get("title", ""),
                "branch": head.get("ref", ""),
                "head": head.get("sha", ""),
                "builder": candidate.builder,
                "pull": pull_text,
                "issue": issue_text,
                "thread": "\n\n".join(p for p in (pull_text, issue_text) if p),
                "bot_pr": True,
                "issue_number": issue_number,
            }
            message = (f"Starting a second review now{run_link(cfg)}, on {who}. The change was "
                       f"built and approved by `{builder}`; it merges only once another model "
                       "approves it too.")
        else:
            source = str(record.get("source") or "request")
            feedback = threads.pull_feedback(ctx.gh, ctx.trust, number, cfg.bot_login,
                                             record.get("feedback_since"), issue=issue_number)
            if source == "ci":
                feedback += "\n\n" + ci_logs(ctx, record.get("ci_run_id"))
            if source == "cross-review" and record.get("last_findings"):
                lines = [f"- `{f.get('where', '?')}`: {f.get('claim', '')} ({f.get('evidence', '')})"
                         for f in record.get("last_findings") or []]
                feedback += "\n\n" + data("\n".join(lines), "The second review's blocking findings")
            planned = {
                "action": "revise",
                "number": number,
                "title": pull.get("title", ""),
                "branch": head.get("ref", ""),
                "source": source,
                "pull": pull_text,
                "issue": issue_text,
                "feedback": feedback,
                "thread": "\n\n".join(p for p in (pull_text, issue_text, feedback) if p),
                "bot_pr": LABEL_PR in names,
                "issue_number": issue_number,
            }
            message = (f"Starting a revision now{run_link(cfg)}, on {who}, because of: {source}. "
                       "It goes through the same checks and adversarial review before I push it.")
    planned.update(_provider_fields(ctx, provider))
    if record.get("handoff"):
        planned["handoff"] = record["handoff"]
    set_state_label(ctx, number, names, LABEL_WORKING)
    taken: list[str] = []
    def change(state: dict[str, Any]) -> None:
        entry = state_item(state, number)
        entry.update(run_id=cfg.run_id, started_at=iso(ctx.now()), kind=kind,
                     stop_requested=False, pending_request=False, provider=provider.id)
        taken[:] = asks.take(entry)
        state["last_run"] = {"at": iso(ctx.now()), "url": cfg.run_url,
                             "what": f"{kind} #{number} on {provider.id}"}
    ctx.store.update(change, f"claim #{number}")
    ctx.gh.create_comment(number, message)
    asks.react(ctx.gh, taken, asks.WORKING)
    return planned


def ci_logs(ctx: Context, run_id: Any) -> str:
    """The tails of the failed jobs of one CI run, fenced as data."""
    if not run_id:
        return data("(no CI run was recorded)", "CI failure")
    try:
        jobs = [j for j in ctx.gh.list_jobs(run_id)
                if j.get("conclusion") in ("failure", "timed_out", "cancelled")]
    except GitHubError as exc:
        return data(f"(could not read the CI run: {exc})", "CI failure")
    parts = []
    for job in jobs[:CI_LOG_JOBS]:
        failed = [s.get("name") for s in job.get("steps", []) if s.get("conclusion") == "failure"]
        parts.append(f"## Job: {job.get('name')} ({job.get('conclusion')})\n"
                     f"Failed steps: {', '.join(str(f) for f in failed) or '?'}\n\n"
                     f"{ctx.gh.job_log(job.get('id'))}")
    return data("\n\n".join(parts) or "(no failed job found)", f"CI run {run_id}")


def open_suggestion_count(ctx: Context) -> int:
    return len([i for i in ctx.gh.list_issues(labels=LABEL_SUGGESTION) if "pull_request" not in i])


def suggestions_due(ctx: Context, state: dict[str, Any], *, force: bool = False) -> int:
    """How many suggestions a survey may open now: 0 when none is due or the cap is full."""
    cfg = ctx.cfg
    if not cfg.suggestions_enabled:
        return 0
    suggest = state.get("suggest") or {}
    last = parse_iso(suggest.get("last_run"))
    due = force or bool(suggest.get("requested")) or last is None or (
        ctx.now() - last >= timedelta(hours=cfg.suggestions_min_interval_hours))
    if not due:
        return 0
    return max(0, cfg.suggestions_max_open - open_suggestion_count(ctx))


def suggestion_plan(ctx: Context, lanes: Lanes, *, force: bool,
                    quiet_ok: str = "") -> dict[str, Any] | None:
    cfg = ctx.cfg
    state = ctx.store.load()
    count = suggestions_due(ctx, state, force=force)
    if count <= 0:
        return None
    provider = survey_provider(ctx, state, lanes, force=force, quiet_ok=quiet_ok)
    if provider is None:
        return None
    lines = []
    for issue in ctx.gh.list_issues(labels="", state="all", limit=150):
        kind = "PR" if "pull_request" in issue else "issue"
        tags = ", ".join(sorted(label_names(issue)))
        lines.append(f"- {kind} #{issue['number']} [{issue.get('state')}] {issue.get('title', '')}"
                     + (f" ({tags})" if tags else ""))
    previous = state.get("suggest") or {}
    was_requested = bool(previous.get("requested"))

    taken: list[str] = []
    def change(s: dict[str, Any]) -> None:
        s["suggest"].update(last_run=iso(ctx.now()), requested=False, provider=provider.id,
                            run_id=cfg.run_id)
        taken[:] = asks.take(s["suggest"])
        s["last_run"] = {"at": iso(ctx.now()), "url": cfg.run_url,
                         "what": f"suggestions on {provider.id}"}
    ctx.store.update(change, "claim suggestions")
    asks.react(ctx.gh, taken, asks.WORKING)
    return {
        "action": "suggest",
        "count": count,
        "existing": data("\n".join(lines) or "(none)", "Existing issues and pull requests"),
        "forced": bool(force),
        "was_requested": was_requested,
        "previous_last_run": previous.get("last_run"),
        **_provider_fields(ctx, provider),
    }


def first(iterable: Iterator[Any]) -> Any:
    return next(islice(iterable, 1), None)
