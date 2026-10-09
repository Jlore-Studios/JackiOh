"""The first job of a night run: decide what this run does and which subscription does it.

It spends nothing. In order: the two kill switches, housekeeping (items a dead run left
`bot:working`, bot PRs that conflict with `main`), the lanes (at most `max_parallel` runs on
GitHub's runners and, apart from those, `machine_parallel` on the machine hold an item at once,
and a subscription holds as many as its `lanes`), then one item and the subscription that
takes it (`pairs`), or a suggestion survey when nothing is queued and one is due. Having claimed
an item, it starts another run when a lane and more work are still free, so the lanes fill up.

Who does what (`pairs`, `assign`): items go in order of urgency (forced, then the priority tier,
reviews, revisions, then builds, the harder first and then the oldest), so work already begun is
finished before new work starts. An item's difficulty (`difficulty:easy`,
`medium` or `hard`; medium without a label) sets the weakest tier that may build it (`MIN_TIER`).
Among the seats (`providers.Seat`: one model on one subscription) whose subscription is free and
available (`providers.availability`: switched on, its secret set, inside its hours unless the item
is forced, under its limits):

- **build, revise, fix**: the first subscription in the usage order (`priority`: claude-3,
  claude-1, then the medium models, then Devin, with `build_last` ones such as claude-2 after
  all of them) with a seat that meets the tier, on its weakest such seat. An easy item goes to
  an `easy_first` subscription (Devin) ahead of that order while it has a free lane; otherwise
  the Claude accounts build it with Sonnet, not Opus (and the run may switch: `work._switch`). A
  builder above the item's tier (no seat of that tier free, or one comes later in the usage
  order) is said in the run's log.
- **plan**: a build that has no plan yet gets a planning session first, on a medium or strong
  seat, strong whenever one is free. When the builder's own subscription has a seat of that tier,
  the plan and the build share one run; otherwise the planning is a run of its own, and the item
  goes back to the queue to build from the plan. Such a planning run is short and starts before
  any long run, so a planner plans Devin's next item before it builds one of its own.
- **review** in the run: the run's own strongest seat of at least medium; a run with none (Devin's,
  which checks itself instead) hands the change to a review run.
- **a review run** (`bot:cross-review`): a strong seat whenever one is free, otherwise a medium one
  (a family that has not approved the head yet first, but the same model may review it twice),
  otherwise, for an easy item that has no weak approval yet, a weak one. What counts is the
  review rule (`review_rule.py`): one strong approval, or two medium ones, or for an easy item one
  weak and one medium.
- **a conflict on a cleared change** (a bot PR whose commit met the rule, then conflicted with
  `main`): a builder with its own reviewer in the run first, since that review carries the
  clearance to the resolution (`deliver._carry`) and no review run follows.

An item labelled `human` is never queued (`candidates`), nor a build that waits for another
issue (`queue.waits_for`: a "Blocked by #n" line, GitHub's own blocked-by link, or an earlier
part of the same patch still open). `claude-1` is shared with its owner, so
an unforced run uses it only after the gate saw it quiet (`quiet_ok`).
"""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import timedelta
from itertools import islice
from typing import Any, Iterator

from harness import asks, issueplan, review_rule, threads
from harness import journal as journal_mod
from harness import providers as providers_mod
from harness.clock import iso, parse_iso
from harness.config import (DIFFICULTIES, HALT_PATH, LABEL_BLOCKED, LABEL_BUILD, LABEL_CROSS,
                            LABEL_NEEDS_PLAN, LABEL_PLANNED, LABEL_PR, LABEL_PR_OPEN, LABEL_REVISE,
                            LABEL_SUGGESTION, LABEL_WORKING, MIN_TIER, NIGHT_WORKFLOW, OTHERS,
                            PART_FLOOR, SLASH, STATE_BRANCH)
from harness.context import Context
from harness.errors import GitHubError
from harness.prompts import data
from harness.providers import TIER_RANK, Pool, Provider, Seat, tier_at_least
from harness.queue import (KIND_ORDER, PRIORITY_NAMES, Candidate, branch_for_issue, candidates,
                           carries, cleared, is_human, label_names, needs_plan,
                           open_pull_for_branch, plan_floor, plan_meets, rating_source,
                           set_state_label)
from harness.state import item as state_item

MODES = ("auto", "build", "revise", "review", "suggest")
#: What an assignment's run does: its action in the plan file.
ACTIONS = ("plan", "build", "revise", "review")
#: A run asked for and never taken up is dropped after this long.
RUN_REQUEST_TTL = timedelta(hours=12)
CI_LOG_JOBS = 4
#: A run in one of these states is still going.
LIVE = ("queued", "in_progress", "waiting", "pending", "requested")
#: The role a provider must allow for each kind of item.
ROLE_OF = {"build": "build", "revise": "revise", "review": "review", "plan": "plan"}
#: `quiet_ok` that lets every provider through, for questions that start nothing.
ANY_QUIET = "*"
#: How long a review run waits for a strong or medium reviewer before a weak one (Devin) may take
#: it (#317 part 9): a weak approval counts only for an easy item, so it rarely moves one.
WEAK_REVIEW_AFTER = timedelta(minutes=30)
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
        return f"halted by {HALT_PATH} on main"
    if state.get("halted"):
        return f"halted by {SLASH} halt"
    if cfg.secrets.known and not any(p.enabled and (p.login == "machine" or cfg.secrets.has(p.secret))
                                     for p in cfg.pool.ordered()):
        return "no subscription has its secret set, so no model can run"
    # A run that could not work backs off its own subscription only (providers.INFRA_BACKOFFS).
    return None


# ---------------------------------------------------------------------- lanes and matching


@dataclass
class Lanes:
    """The runs that hold work now: item number (0 for a suggestion survey) -> provider id.
    `planning` are the items whose run is a planning run: those hold the planning lane
    (`Pool.plan_lanes`), not a build lane, nor a lane of their provider's. GitHub's runners and
    the bot's machine have lanes of their own, so neither waits on the other: `limit` is
    GitHub's (`Pool.max_parallel`), `machine_limit` the machine's (`Pool.machine_parallel`), and
    `machine` names the subscriptions that run on the machine."""

    limit: int
    held: dict[int, str] = field(default_factory=dict)
    planning: set[int] = field(default_factory=set)
    plan_limit: int = 0
    machine_limit: int = 0
    machine: frozenset[str] = frozenset()

    @property
    def busy(self) -> set[str]:
        return set(self.held.values())

    def count(self, provider_id: str) -> int:
        """How many runs this provider holds now, its planning runs aside."""
        return sum(1 for number, held in self.held.items()
                   if held == provider_id and number not in self.planning)

    def planning_by(self, provider_id: str) -> int:
        """How many planning runs this provider holds now."""
        return sum(1 for number in self.planning if self.held.get(number) == provider_id)

    def full(self, provider: Provider) -> bool:
        """It holds as many runs as its `lanes`. A subscription the quiet check guards, or one
        with usage caps, counts its planning run too: the quiet check cannot tell a second run of
        the bot's from its owner, and two runs deciding from one reading go past a cap together
        (claude-2 planned and revised at once from 0% and was refused at 100% 15 minutes later)."""
        held = self.count(provider.id)
        if provider.quiet_check or provider.limits.stops:
            held += self.planning_by(provider.id)
        return held >= provider.lanes

    def on_machine(self, pool: providers_mod.Pool) -> int:
        """How many of the held runs are on the bot's machine."""
        return sum(1 for provider_id in self.held.values() if pool.on_machine(provider_id))

    @property
    def free(self) -> int:
        """Build lanes free on GitHub's runners."""
        return max(0, self.limit - sum(1 for number, provider_id in self.held.items()
                                       if number not in self.planning
                                       and provider_id not in self.machine))

    @property
    def machine_free(self) -> int:
        """Slots free on the machine; a planning run there takes one too (`machine_full`)."""
        return max(0, self.machine_limit - sum(1 for provider_id in self.held.values()
                                               if provider_id in self.machine))

    def room_for(self, provider_id: str) -> bool:
        """A lane is free where `provider_id` runs: the machine, or GitHub's runners."""
        return self.machine_free > 0 if provider_id in self.machine else self.free > 0

    @property
    def any_room(self) -> bool:
        return self.free > 0 or self.machine_free > 0 or self.plan_free > 0

    @property
    def plan_free(self) -> int:
        return max(0, self.plan_limit - len(self.planning))

    def with_run(self, number: int, provider_id: str, action: str) -> "Lanes":
        """These lanes with one more run held."""
        planning = self.planning | ({number} if action == "plan" else set())
        return Lanes(self.limit, {**self.held, number: provider_id}, planning, self.plan_limit,
                     self.machine_limit, self.machine)

    def describe(self) -> str:
        return ", ".join((f"#{n} with `{p}`" + (" (planning)" if n in self.planning else ""))
                         if n else f"a survey with `{p}`"
                         for n, p in sorted(self.held.items())) or "none"


def working_threads(ctx: Context) -> list[dict[str, Any]]:
    """Every thread labelled `bot:working`, closed ones too: a run goes on after someone closes
    its issue, and holds its lane until it ends (deliver then drops its work)."""
    return ctx.gh.list_issues(labels=LABEL_WORKING, state="all")


def read_lanes(ctx: Context, state: dict[str, Any]) -> Lanes:
    """The lanes held by runs that are still going. A run that ended without delivering holds
    nothing: housekeeping requeues its item."""
    pool = ctx.cfg.pool
    lanes = Lanes(pool.max_parallel, plan_limit=pool.plan_lanes,
                  machine_limit=pool.machine_parallel,
                  machine=frozenset(p.id for p in pool.ordered() if pool.on_machine(p.id)))
    for thread in working_threads(ctx):
        number = int(thread["number"])
        record = state["items"].get(str(number), {})
        if run_alive(ctx, record.get("run_id")):
            lanes.held[number] = str(record.get("provider") or providers_mod.LEGACY_PROVIDER)
            if record.get("action") == "plan":
                lanes.planning.add(number)
    survey = state.get("suggest") or {}
    if survey.get("provider") and run_alive(ctx, survey.get("run_id")):
        lanes.held[0] = str(survey["provider"])
    return lanes


def _usable(ctx: Context, state: dict[str, Any], provider: Provider, lanes: Lanes, role: str,
            *, forced: bool, quiet_ok: str) -> bool:
    cfg = ctx.cfg
    if lanes.full(provider) or role not in provider.roles or not lanes.room_for(provider.id):
        return False
    if machine_full(cfg.pool, provider, lanes):
        return False
    if (provider.quiet_check and cfg.quiet.enabled and not forced
            and quiet_ok not in (ANY_QUIET, provider.id)):
        return False
    # A build or a revision runs long: it starts only with `start_headroom` under each cap.
    return providers_mod.availability(provider, state, ctx.now(), cfg.timezone, cfg.secrets,
                                      forced=forced, starting=role in ("build", "revise")) is None


@dataclass
class Assignment:
    """One run's work: its subscription, and the seat each role runs on there. `action` is
    `plan` (a planning run of its own), `build`, `revise` or `review` (a review run)."""

    provider: Provider
    action: str
    difficulty: str
    #: A planning session before the build, in this run.
    plan: Seat | None = None
    #: The builder (and its fixes).
    build: Seat | None = None
    #: The reviewer: in a build or revise run, its own adversarial reviewer (None hands the change
    #: to a review run); in a review run, the one reviewing.
    review: Seat | None = None
    #: What the router decided that a reader should know: a step up in tier, and why.
    notes: list[str] = field(default_factory=list)

    @property
    def self_check(self) -> bool:
        return bool(self.build and self.build.self_check)

    def describe(self) -> str:
        parts = []
        for role, seat in (("plan", self.plan), ("build", self.build), ("review", self.review)):
            if seat is not None:
                parts.append(f"{role} on {seat.describe()}")
        if self.build is not None and self.review is None and self.action != "review":
            parts.append("review in a review run" + (" after its self check"
                                                       if self.self_check else ""))
        return "; ".join(parts) or f"on {self.provider.describe()}"

    def seats(self) -> dict[str, Any]:
        """What the model job runs each role on (`work.py`)."""
        return {"plan": self.plan.to_dict() if self.plan else None,
                "build": self.build.to_dict() if self.build else None,
                "review": self.review.to_dict() if self.review else None,
                "self_check": self.self_check}


def free_providers(ctx: Context, state: dict[str, Any], lanes: Lanes, role: str, *,
                   forced: bool, quiet_ok: str) -> list[Provider]:
    return [p for p in ctx.cfg.pool.ordered()
            if _usable(ctx, state, p, lanes, role, forced=forced, quiet_ok=quiet_ok)]


def ranked(pool: Pool, seats: list[Seat]) -> list[Seat]:
    """`seats` in the order the router tries them: the usage order (`priority`), then the
    tier's own preference order."""
    order = {name: i for i, name in enumerate(pool.priority)}
    return sorted(seats, key=lambda seat: (order.get(seat.provider.id, len(order)), seat.rank))


def builder_seat(pool: Pool, providers: list[Provider], difficulty: str, *,
                 part: bool = False) -> tuple[Seat | None, str]:
    """Who builds an item of `difficulty`: the first free subscription in the usage order with a
    seat that meets its tier, on its weakest such seat; with a note when that seat is above the
    tier, saying why (none of that tier is free, or the usage order puts this one first).

    A fullsend `part` (#505), whose plan is on record by now, goes first to the subscriptions
    whose strongest seat is weakest, and one with no seat of its tier may build it on a seat of
    `PART_FLOOR`: Muse builds a hard part from Opus's plan, and the strong models keep to the
    planning and to the reconcile that checks and reviews every part."""
    floor = MIN_TIER[difficulty]
    free = {provider.id for provider in providers}

    def strongest(provider: Provider) -> int:
        seat = pool.best_seat(provider)
        return TIER_RANK[seat.tier] if seat is not None else 0
    # An easy item goes first to a subscription marked `easy_first` (Devin), which may build
    # nothing harder; one marked `build_last` (claude-2) builds only after every other one.
    order = sorted(providers, key=lambda provider: (
        not (provider.easy_first and difficulty == "easy"), strongest(provider) if part else 0,
        provider.build_last))

    def builds(provider: Provider, seat: Seat) -> bool:
        # A stand-in (`takes_over`, Sonnet for Devin) builds only an easy item, and only while
        # the subscription it stands in for cannot take it (#317 part 9).
        if seat.takes_over and (difficulty != "easy" or seat.takes_over in free):
            return False
        if tier_at_least(seat.tier, floor):
            return True
        return (part and tier_at_least(seat.tier, PART_FLOOR)
                and pool.best_seat(provider, floor) is None)
    usable = [(provider, [seat for seat in pool.seats(provider) if builds(provider, seat)])
              for provider in order]
    for index, (provider, seats) in enumerate(usable):
        if not seats:
            continue
        seat = min(seats, key=lambda s: (TIER_RANK[s.tier], s.rank))
        if not tier_at_least(seat.tier, floor):
            return seat, (f"a fullsend part, built on {seat.tier} from its plan: its tree's "
                          "reconcile checks and reviews it")
        if seat.tier == floor:
            return seat, ""
        later = [s.describe() for _, others in usable[index + 1:] for s in others
                 if s.tier == floor]
        if later:
            return seat, (f"built on {seat.tier} though difficulty:{difficulty} allows {floor}: "
                          f"the usage order puts `{provider.id}` before {', '.join(later)}")
        models = ", ".join(e.model for e in pool.tiers.get(floor, ())) or "none listed"
        return seat, (f"stepped up from {floor} to {seat.tier} to build it: no {floor}-tier "
                      f"model ({models}) is free now")
    return None, ""


def lane_planners(ctx: Context, state: dict[str, Any], lanes: Lanes, *, forced: bool,
                  quiet_ok: str) -> list[Seat]:
    """The planning lane's planners: the strongest seat of each subscription that may plan now
    (at least medium), the strong ones first, and within a tier the one holding the fewest plans
    first, then the usage order. A planning run there takes no build lane, so a subscription plans
    while it builds, as many items at once as it has lanes: planning comes first, so the strong
    models write the plans the medium ones build from before they build themselves. One the quiet
    check guards, or one with caps, plans only when it holds nothing else. Which item a planner
    may take is the item's plan floor (`queue.plan_floor`): a medium one plans only easy or
    unrated items. A plan is no short call (an Opus planner spent 20 minutes on #37), so like a
    build it starts only `start_headroom` under each cap."""
    if lanes.plan_free <= 0:
        return []
    cfg = ctx.cfg
    seats = []
    for provider in cfg.pool.ordered():
        seat = cfg.pool.best_seat(provider, "medium")
        if seat is None or "plan" not in provider.roles:
            continue
        planning = lanes.planning_by(provider.id)
        if planning >= provider.lanes:
            continue
        if (provider.quiet_check or provider.limits.stops) and (planning or lanes.count(provider.id)):
            continue  # one run at a time on a guarded or capped subscription (`Lanes.full`)
        if machine_full(cfg.pool, provider, lanes):
            continue
        if (provider.quiet_check and cfg.quiet.enabled and not forced
                and quiet_ok not in (ANY_QUIET, provider.id)):
            continue
        if providers_mod.availability(provider, state, ctx.now(), cfg.timezone, cfg.secrets,
                                      forced=forced, starting=True) is None:
            seats.append(seat)
    return sorted(ranked(cfg.pool, seats), key=lambda seat: (
        -TIER_RANK[seat.tier], lanes.planning_by(seat.provider.id)))


def run_reviewer(pool: Pool, provider: Provider, difficulty: str) -> Seat | None:
    """A build or revise run's own adversarial reviewer: its strongest seat of at least medium,
    or None, and the change goes to a review run. (`difficulty` does not change it: a hard item's
    builder is strong, so its own reviewer is too.)"""
    if "review" not in provider.roles:
        return None
    return pool.best_seat(provider, "medium")


def review_seat(pool: Pool, providers: list[Provider], candidate: Candidate,
                waited: timedelta | None = None) -> Seat | None:
    """A review run's reviewer: strong whenever one is free; otherwise medium, a family that has
    not approved the head yet before one that has (the same model may review it twice);
    otherwise weak, only where a weak approval helps (`review_rule.helps`: an easy item with no
    weak approval yet), and only once the review has waited `WEAK_REVIEW_AFTER` for a stronger
    one (`waited`; None does not wait). A stand-in seat (`takes_over`) reviews nothing."""
    seats = [seat for provider in providers for seat in pool.own_seats(provider)]
    for tier in ("strong", "medium", "weak"):
        if candidate.review_floor and not tier_at_least(tier, candidate.review_floor):
            continue  # a person asked for this tier or stronger (`/harness review strong`)
        if not review_rule.helps(candidate.approval_tiers, tier, candidate.difficulty):
            continue
        if tier == "weak" and waited is not None and waited < WEAK_REVIEW_AFTER:
            continue
        found = ranked(pool, [seat for seat in seats if seat.tier == tier])
        found.sort(key=lambda seat: seat.family in candidate.approved)  # stable: keeps the order
        if found:
            return found[0]
    return None


def assign(ctx: Context, state: dict[str, Any], candidate: Candidate, lanes: Lanes, *,
           force: bool, quiet_ok: str) -> Assignment | None:
    """Who takes `candidate` now, and on which seats, or None when nobody can."""
    pool = ctx.cfg.pool
    forced = force or candidate.forced

    def free(role: str) -> list[Provider]:
        return free_providers(ctx, state, lanes, role, forced=forced, quiet_ok=quiet_ok)

    if candidate.kind == "review":
        queued = parse_iso(candidate.queued_at)
        waited = ctx.now() - queued if queued is not None else None
        seat = review_seat(pool, free("review"), candidate, waited)
        if seat is None:
            return None
        return Assignment(seat.provider, "review", candidate.difficulty, review=seat)
    builders = free(ROLE_OF[candidate.kind])
    floor = plan_floor(candidate)
    if candidate.kind == "build" and not plan_meets(candidate):
        # It has no plan its difficulty may build from (#317 part 6): the planning lane writes one
        # (the Needs plan stage), or a builder whose own seat meets the plan floor plans it first
        # in its run. A builder that cannot plan (Devin) waits for the plan.
        builders = [p for p in builders if "plan" in p.roles
                    and pool.best_seat(p, floor) is not None]
    if candidate.kind == "revise" and (not candidate.bot_pr or candidate.conflict):
        # A person's pull request never gets a review run: its reviewer must be in this run. A
        # conflict with `main` lands in SPEC §11 and the rulings index nearly every time, which a
        # weak builder resolves badly (#203, #214, #287): only a builder that reviews in its own
        # run takes one (#317 part 2), and none at all may wait for one.
        builders = [p for p in builders if run_reviewer(pool, p, candidate.difficulty)]
    builder, note = builder_seat(pool, builders, candidate.difficulty,
                                 part=candidate.part and plan_meets(candidate))
    if builder is None:
        return None
    notes = [f"#{candidate.number}: {note}"] if note else []
    reviewer = run_reviewer(pool, builder.provider, candidate.difficulty)
    if candidate.kind == "build" and not plan_meets(candidate):
        # No planner was free on the planning lane: the builder plans it first in its own run,
        # on its strongest model, which meets the item's plan floor.
        own = pool.best_seat(builder.provider, floor) if "plan" in builder.provider.roles else None
        if own is None:
            return None
        return Assignment(builder.provider, "build", candidate.difficulty, plan=own,
                          build=builder, review=reviewer, notes=notes)
    return Assignment(builder.provider, candidate.kind, candidate.difficulty, build=builder,
                      review=reviewer, notes=notes)


def pairs(ctx: Context, state: dict[str, Any], queue: list[Candidate], lanes: Lanes, *,
          force: bool, quiet_ok: str,
          skipped: list[str] | None = None) -> Iterator[tuple[Candidate, Assignment]]:
    """Every queued item that a subscription can take now, with who takes it on which seats,
    most urgent first. (`skipped` is the queue's: the items labelled `human`.)"""
    # Reviews and revisions finish work already begun, so they go before new builds; among the
    # builds the harder first, since only the stronger models can take them.
    # A review run unblocks a merge and is short, so it goes before every build or revision,
    # whatever their priority (#317 part 9); then the priority tier and the usual order.
    order = sorted(queue, key=lambda c: (not (force or c.forced), c.kind != "review", c.priority,
                                         KIND_ORDER[c.kind], -DIFFICULTIES.index(c.difficulty),
                                         c.queued_at, c.number))
    # The Needs plan stage first: a planner on the planning lane, which takes no build lane,
    # plans the items a builder that cannot plan (Devin) waits on before the rest, then the
    # fullsend parts a medium subscription (Muse) builds from their plans, each by a model that
    # meets its plan floor (medium for an easy or unrated item, strong for the rest).
    planning: list[tuple[Candidate, Assignment]] = []
    planners = lane_planners(ctx, state, lanes, forced=force, quiet_ok=quiet_ok)
    if planners:
        for candidate in sorted((c for c in order if needs_plan(c)),
                                key=lambda c: (not (force or c.forced), c.difficulty != "easy",
                                               not c.part)):
            seat = next((s for s in planners if tier_at_least(s.tier, plan_floor(candidate))),
                        None)
            if seat is None:
                continue
            what = "rates and plans it" if not candidate.rated else "writes it"
            planning.append((candidate, Assignment(
                seat.provider, "plan", candidate.difficulty, plan=seat,
                notes=[f"#{candidate.number}: needs a plan; {seat.describe()} {what} on the "
                       "planning lane"])))
    found: list[tuple[Candidate, Assignment]] = []
    if lanes.free > 0 or lanes.machine_free > 0:
        found = [(candidate, assignment) for candidate in order
                 if (assignment := assign(ctx, state, candidate, lanes, force=force,
                                          quiet_ok=quiet_ok)) is not None]
    planning.sort(key=lambda pair: not (force or pair[0].forced))
    yield from planning
    yield from found


def survey_provider(ctx: Context, state: dict[str, Any], lanes: Lanes, *, force: bool,
                    quiet_ok: str) -> Provider | None:
    for provider in ctx.cfg.pool.ordered():
        if _usable(ctx, state, provider, lanes, "suggest", forced=force, quiet_ok=quiet_ok):
            return provider
    return None


def machine_full(pool: providers_mod.Pool, provider: Provider, lanes: Lanes) -> bool:
    """`provider` runs on the bot's machine (the night box), and its `machine_parallel` slots are
    all held. GitHub's runners have `max_parallel` of their own. (The AI's training lanes
    run on a box of their own, outside the harness, so no slot is kept for them.)"""
    if not pool.on_machine(provider.id):
        return False
    held = sum(1 for held_id in lanes.held.values() if pool.on_machine(held_id))
    return held >= pool.machine_parallel


def why_none(ctx: Context, state: dict[str, Any], lanes: Lanes) -> str:
    """Why each subscription cannot take work now, for the run's summary and `status`."""
    cfg = ctx.cfg
    parts = []
    for provider in cfg.pool.ordered():
        if lanes.full(provider):
            parts.append(f"`{provider.id}` is busy")
            continue
        # What an idle subscription waits for is a build, a revision or a plan, which start only
        # `start_headroom` under each cap (`_usable`, `lane_planners`).
        reason = providers_mod.start_reason(provider, state, ctx.now(), cfg.timezone, cfg.secrets)
        if reason is None and machine_full(cfg.pool, provider, lanes):
            reason = f"waits for room on the machine ({cfg.pool.machine_parallel} at once)"
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
    #: Queued items passed over because of `human` (#96), one line each.
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
    if not lanes.any_room:
        if tidy:
            return Peek(True, tidy, force, held=len(lanes.held))
        return Peek(False, f"every lane is busy ({lanes.describe()})", held=len(lanes.held))
    queue = _queue(ctx, state, item, mode, skipped)
    found: tuple[Candidate | None, Provider, bool] | None = None
    chosen: Assignment | None = None
    if mode != "suggest":
        top = next(pairs(ctx, state, queue, lanes, force=force, quiet_ok=ANY_QUIET,
                         skipped=skipped), None)
        if top is not None:
            chosen = top[1]
            found = (top[0], top[1].provider, force or top[0].forced)
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
    what = (f"#{candidate.number} ({candidate.difficulty}) is queued to {candidate.kind}"
            if candidate else "a suggestion survey is due")
    reason = f"{what}, for `{provider.id}`" + (f": {chosen.describe()}" if chosen else "")
    if not (provider.quiet_check and ctx.cfg.quiet.enabled and not forced):
        return Peek(True, reason, forced, provider.id, held=len(lanes.held))
    # The best subscription for it is the shared one: wait for quiet, unless another run is
    # already waiting, and say whether other work could go ahead without it.
    other = next(pairs(ctx, state, queue, lanes, force=force, quiet_ok=""), None) if (
        mode != "suggest") else None
    fallback = other is not None
    if other is not None and other[1].provider.family == provider.family:
        # Another account of the same model can take work now: no reason to wait for this one.
        return Peek(True, f"#{other[0].number} ({other[0].difficulty}) is queued to "
                    f"{other[0].kind}, for `{other[1].provider.id}`: {other[1].describe()}",
                    force or other[0].forced, other[1].provider.id, held=len(lanes.held))
    if quiet_waiting(ctx):
        if fallback:
            return Peek(True, f"#{other[0].number} ({other[0].difficulty}) is queued to "
                        f"{other[0].kind}, for `{other[1].provider.id}` (another run waits for "
                        f"`{provider.id}`)", forced, other[1].provider.id,
                        held=len(lanes.held))
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


def announce_switched_off(ctx: Context, state: dict[str, Any]) -> list[str]:
    """On the day a subscription's `off_from` comes, open one issue asking a person what it
    should do now (the bot cannot change `.harness/`), and remember that it did."""
    notes: list[str] = []
    cfg = ctx.cfg
    for provider in cfg.pool.ordered():
        if not provider.enabled or not providers_mod.switched_off_by_date(
                provider, ctx.now(), cfg.timezone):
            continue
        if providers_mod.peek_record(state, provider.id).get("off_announced"):
            continue
        reason = f"\n\n{provider.off_reason}" if provider.off_reason else ""
        issue = ctx.gh.create_issue(
            f"Night bot: `{provider.id}` is switched off from {provider.off_from}; decide what it "
            "should do now",
            f"`.harness/providers.json` gives `{provider.id}` ({provider.cli}, `{provider.model}`) "
            f"`off_from: {provider.off_from}`, so from that day the night bot starts no new work "
            f"on it.{reason}\n\nTo decide, in a pull request a person makes (the bot cannot "
            "change `.harness/`):\n- keep it off: remove its entry, or set `\"enabled\": false`;"
            "\n- point it at another model, or give it caps under `limits`;\n- or move `off_from` "
            "later.", labels=["night bot"])
        number = int(issue.get("number") or 0)
        ctx.store.update(lambda s, p=provider.id, n=number: providers_mod.record(s, p).update(
            off_announced=n), f"{provider.id} switched off")
        notes.append(f"`{provider.id}` is switched off from {provider.off_from}: opened #{number}")
    return notes


def housekeeping_due(ctx: Context, state: dict[str, Any]) -> str | None:
    """What `housekeeping` would requeue, read without changing anything, or None."""
    for thread in working_threads(ctx):
        run_id = state["items"].get(str(thread["number"]), {}).get("run_id")
        if run_status(ctx, run_id) == "dead":
            return f"#{thread['number']} was left working by a run that ended"
    for thread in ctx.gh.list_issues(labels=LABEL_PR):
        names = label_names(thread)
        if ("pull_request" not in thread or names & {LABEL_REVISE, LABEL_WORKING, LABEL_BLOCKED}
                or is_human(names)):
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
    note_secrets(ctx, state)
    notes = housekeeping(ctx, state) + announce_switched_off(ctx, state)

    def taken(planned: dict[str, Any]) -> dict[str, Any]:
        # A `/harness run` is taken up by the run that claims something for it, never by one
        # that found every lane busy: one request, one run.
        if asked is not None:
            ctx.store.update(lambda s: s.update(run_requested=None), "run request taken")
        return planned
    state = ctx.store.load()
    notes += sync_needs_plan(ctx, state)
    lanes = read_lanes(ctx, state)
    if not lanes.any_room:
        return {**nothing(f"every lane is busy ({lanes.describe()})"), "housekeeping": notes}
    skipped: list[str] = []
    queue = _queue(ctx, state, item, mode, skipped)
    if mode != "suggest":
        for candidate, assignment in pairs(ctx, state, queue, lanes, force=force,
                                           quiet_ok=quiet, skipped=skipped):
            planned = claim(ctx, candidate, assignment)
            if planned is not None:
                planned["housekeeping"] = notes
                planned["forced"] = bool(force or candidate.forced)
                planned["priority"] = PRIORITY_NAMES[candidate.priority]
                planned["skipped"] = skipped
                fill_lanes(ctx, lanes, candidate, assignment, planned)
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


def fill_lanes(ctx: Context, lanes: Lanes, taken: Candidate, assignment: Assignment,
               planned: dict[str, Any]) -> None:
    """Start one more run when, with this item claimed, a lane (or the planning lane) and more
    work are still free. That run does the same, so the lanes fill one run at a time."""
    after = lanes.with_run(taken.number, assignment.provider.id, assignment.action)
    if not after.any_room:
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


def sync_needs_plan(ctx: Context, state: dict[str, Any]) -> list[str]:
    """Keep `bot:needs-plan` on exactly the queued items in the Needs plan stage, and on the ones
    being planned now; and `bot:planned` on the queued ones a strong model has planned (it stays
    after they leave the queue, and comes off only if the item needs a plan again)."""
    notes: list[str] = []
    try:
        queued = [c for c in candidates(ctx, state) if c.kind == "build"]
        wanted = {c.number for c in queued if needs_plan(c)}
        labelled = ctx.gh.list_issues(labels=LABEL_NEEDS_PLAN)
        # A mode (#60) plans for itself: its item carries no `planned` label.
        planned = {c.number for c in queued if c.planned and not needs_plan(c) and not c.mode}
        has_planned = {int(t["number"]) for t in ctx.gh.list_issues(labels=LABEL_PLANNED)}
    except GitHubError as exc:
        return [f"could not sync `{LABEL_NEEDS_PLAN}`: {exc}"]
    for number in sorted(planned - has_planned):
        ctx.gh.add_labels(number, [LABEL_PLANNED])
    for number in sorted(has_planned & wanted):
        ctx.gh.remove_label(number, LABEL_PLANNED)
    have = set()
    for thread in labelled:
        number = int(thread["number"])
        have.add(number)
        record = state["items"].get(str(number), {})
        planning = LABEL_WORKING in label_names(thread) and record.get("action") == "plan"
        if number not in wanted and not planning:
            ctx.gh.remove_label(number, LABEL_NEEDS_PLAN)
            notes.append(f"#{number} left the Needs plan stage")
    for number in sorted(wanted - have):
        ctx.gh.add_labels(number, [LABEL_NEEDS_PLAN])
        notes.append(f"#{number} needs a plan")
    return notes


def note_secrets(ctx: Context, state: dict[str, Any]) -> None:
    """Record which provider secrets this run's workflow has, for the status loop: its own list
    is fixed when its long run is created, so a secret added since shows there once a plan job
    has seen it (`providers.newer_secrets`)."""
    record = providers_mod.secrets_record(ctx.cfg.secrets, state, ctx.now())
    if record is not None:
        ctx.store.update(lambda s: s.update(secrets=record), "secrets seen")


#: How long an item's record outlives its closed thread in the state file, and how often the
#: records are looked over (#160, #317 part 12): GitHub's Contents API stops returning a file past
#: 1 MB, and the 95 item records held 303 KB of the 374 KB file on 2026-10-05.
KEEP_CLOSED = timedelta(days=7)
PRUNE_EVERY = timedelta(hours=24)
#: What a closed thread's record sheds at once: the bulky parts only an open one uses.
SHED_WHEN_CLOSED = ("handoff", "last_findings", "self_check_findings", "strike_log", "votes",
                    "asks", "taken_asks", "question")


def prune(ctx: Context, state: dict[str, Any]) -> list[str]:
    """Once a day: drop the record of every item whose thread closed more than `KEEP_CLOSED`
    ago, and the bulky parts of the others that are closed. Open threads keep everything."""
    last = parse_iso((state.get("pruned") or {}).get("at"))
    now = ctx.now()
    if last is not None and now - last < PRUNE_EVERY:
        return []
    try:
        open_now = {int(t["number"]) for t in ctx.gh.list_issues(state="open", limit=1000)}
    except GitHubError as exc:
        return [f"could not prune the state file: {exc}"]
    gone: list[int] = []
    shed: list[int] = []
    for key in list(state["items"]):
        if not key.isdigit() or int(key) in open_now or not int(key):
            continue
        try:
            thread = ctx.gh.get_issue(int(key))
        except GitHubError:
            continue
        if thread.get("state") != "closed":
            continue
        closed = parse_iso(thread.get("closed_at"))
        if closed is not None and now - closed >= KEEP_CLOSED:
            gone.append(int(key))
        elif any(field in state["items"][key] for field in SHED_WHEN_CLOSED):
            shed.append(int(key))

    def change(s: dict[str, Any]) -> None:
        for number in gone:
            s["items"].pop(str(number), None)
        for number in shed:
            for field_name in SHED_WHEN_CLOSED:
                s["items"].get(str(number), {}).pop(field_name, None)
        s["pruned"] = {"at": iso(now), "dropped": len(gone), "shed": len(shed)}
    ctx.store.update(change, "prune")
    return [f"pruned the state file: dropped {len(gone)} closed item(s), lightened {len(shed)}"]


def housekeeping(ctx: Context, state: dict[str, Any]) -> list[str]:
    """Requeue items a dead run left working; queue a revision for conflicted bot PRs."""
    notes: list[str] = prune(ctx, state)
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
        if names & {LABEL_REVISE, LABEL_WORKING, LABEL_BLOCKED} or is_human(names):
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
    # `shared` names the run's model step "Build, check and review", which the partner bot
    # reads as this bot spending the shared subscription, so it excuses the rise. claude-1 is
    # that subscription whether or not this bot waits for it to be quiet (`quiet_check`).
    shared = provider.quiet_check or provider.id == "claude-1"
    return {"provider": provider.id, "cli": provider.cli, "secret": provider.secret,
            "family": provider.family, "shared": shared, "vault": vault or "",
            "login": provider.login, "runs_on": provider.runs_on}


def _start_message(assignment: Assignment, cfg: Any) -> str:
    """What a build or revision run says it does, by who runs each role."""
    steps = []
    if assignment.plan is not None:
        steps.append(f"plan it on {assignment.plan.describe()}")
    steps.append(f"build it on {assignment.build.describe()}" if assignment.build else "build it")
    steps.append("run the repository's checks")
    if assignment.self_check:
        steps.append(f"have the builder check its own change, up to {cfg.max_self_check_rounds} "
                     "times")
    if assignment.review is not None:
        steps.append(f"have {assignment.review.describe()} review the change adversarially, up "
                     f"to {cfg.max_review_cycles} rounds")
        end = "Only an approved change becomes a pull request."
    else:
        end = ("No model here reviews it, so the pull request then waits for a review run, until "
               f"{review_rule.SUMMARY} approved it.")
    return f"I {', '.join(steps[:-1])} and {steps[-1]}. {end}"


def claim(ctx: Context, candidate: Candidate,
          assignment: Assignment) -> dict[str, Any] | None:
    """Build the plan for one queued thread on one subscription, and mark it `bot:working`."""
    cfg = ctx.cfg
    provider = assignment.provider
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
            "action": assignment.action,
            "number": number,
            "title": thread.get("title", ""),
            "branch": branch_for_issue(number),
            "thread": threads.issue_thread(ctx.gh, ctx.trust, number, cfg.bot_login),
            "previous_findings": record.get("last_findings") or [],
            "previous_question": record.get("question") or "",
        }
        written = issueplan.plan_of(thread.get("body"))
        if written and assignment.action == "build":
            # A person may have edited the plan in the description: the builder starts from that.
            planned["plan_in_issue"] = written
        # Who rated it, for the planner's prompt and for deliver (#317 part 8).
        by = record.get("difficulty_by") if isinstance(record.get("difficulty_by"), dict) else {}
        planned["rating"] = {"difficulty": candidate.difficulty,
                             "source": rating_source(names, record),
                             "by": str(by.get("provider") or "")}
        if candidate.mode:
            planned["mode"] = candidate.mode
            if candidate.mode.startswith("split") or candidate.mode == "fullsend":
                planned["children"] = tree_text(ctx, number)
                planned["builders"] = split_builders(candidate.mode)
            if candidate.mode == "fullsend":
                # The parts its reconcile merges first: those that could not land (#505).
                planned["parked"] = [str(b) for b in record.get("parked") or []]
        if record.get("onto"):
            # A fullsend part (#505): it lands on its tree's branch, not `main`.
            planned["onto"] = str(record["onto"])
            planned["part_of"] = int(record.get("part_of") or 0)
        if record.get("previous_pr"):
            planned["previous_pr"] = record["previous_pr"]
            planned["previous_branch"] = str(record.get("previous_branch") or "")
            planned["previous_why"] = str(record.get("previous_why") or "")
        if candidate.mode == "fullsend" and planned.get("children"):
            message = (f"Starting work on this now{run_link(cfg)}: the reconcile of this fullsend "
                       f"tree. Every part has closed, so this run merges them on "
                       f"`{planned['branch']}`, follows fullsend's reconcile and makes every check "
                       "green, for one pull request into `main`. "
                       f"{_start_message(assignment, cfg)}")
        elif candidate.mode == "fullsend":
            message = (f"Splitting this now for fullsend{run_link(cfg)}, on "
                       f"{assignment.build.describe()}: one session reads it and the code and "
                       "answers with parts that each own their files. When the run ends I open "
                       f"them, and each lands on `{planned['branch']}`, not `main`, with no pull "
                       "request of its own; once they have all closed, one run reconciles them "
                       "into one pull request.")
        elif candidate.mode.startswith("split"):
            message = (f"Splitting this now{run_link(cfg)}, on {assignment.build.describe()}: "
                       "one session reads it and the code and answers with sub-issues, each small "
                       f"enough for one run. When the run ends I open them, queued for "
                       f"{split_builders(candidate.mode)} in the order they depend on each other.")
        elif planned.get("onto") and assignment.action == "build":
            message = (f"Starting work on this now{run_link(cfg)}: a fullsend part, "
                       f"difficulty:{candidate.difficulty}, built on "
                       f"{assignment.build.describe()}. It lands on `{planned['onto']}`, the "
                       f"branch of #{planned['part_of']}, with no checks, review or pull request "
                       "of its own: the tree's reconcile checks and reviews every part at once.")
        elif candidate.mode == "oneshot":
            message = (f"Starting a one-shot build of this now{run_link(cfg)}, on "
                       f"{assignment.build.describe()}, with fullsend: a spec first, then many "
                       "agents writing its parts at once, each in its own worktree and branch, "
                       "reconciled against tests written from the spec. "
                       f"{_start_message(assignment, cfg)}")
        elif assignment.action == "plan":
            message = (f"Planning this now{run_link(cfg)}, on {assignment.plan.describe()} "
                       f"(`{LABEL_NEEDS_PLAN}`): it is difficulty:{candidate.difficulty}. The plan "
                       "goes into this issue's description, under **Plan**, and the builder starts "
                       "from it; then it is queued to build on the cheapest model its difficulty "
                       "allows.")
        else:
            message = (f"Starting work on this now{run_link(cfg)}: it is "
                       f"difficulty:{candidate.difficulty}. {_start_message(assignment, cfg)}")
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
                "approved": list(candidate.approved),
                "pull": pull_text,
                "issue": issue_text,
                "thread": "\n\n".join(p for p in (pull_text, issue_text) if p),
                "bot_pr": True,
                "issue_number": issue_number,
                "review_notes": str(record.get("review_notes") or ""),
            }
            message = (f"Starting a review now{run_link(cfg)}, on "
                       f"{assignment.review.describe()}. `{builder}` built the change; it merges "
                       f"once {review_rule.SUMMARY} approved the same commit.")
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
            message = (f"Starting a revision now{run_link(cfg)}, because of: {source}. "
                       f"{_start_message(assignment, cfg)}")
            if LABEL_PR in names and carries(record):
                # The work job tells its builder and reviewer; deliver decides from git alone.
                planned["cleared"] = cleared(record)
                if assignment.review is not None:
                    message += (" The reviews had cleared it before `main` moved, so if this "
                                "revision changes nothing but the conflicted files and its "
                                "reviewer approves, it merges without another review run.")
    planned.update(_provider_fields(ctx, provider))
    planned.update(difficulty=candidate.difficulty, seats=assignment.seats(),
                   routing=list(assignment.notes), assignment=assignment.describe())
    if kind == "review" and record.get("self_check_findings"):
        planned["self_check_findings"] = record["self_check_findings"]
    if record.get("handoff"):
        planned["handoff"] = record["handoff"]
    # Every earlier run on the item, for the work job to put in its worktree (#342).
    journal = journal_mod.read(ctx.gh, journal_mod.key(planned) or number)
    if journal:
        planned["journal"] = journal
    if kind == "revise" and isinstance(record.get("wip"), dict):
        planned["wip"] = record["wip"]  # the last cut-off revision's work (#317 part 3)
    if planned.get("plan_in_issue"):
        handoff = planned.get("handoff") if isinstance(planned.get("handoff"), dict) else {}
        if not handoff or handoff.get("kind") in ("plan", "draft"):
            if handoff.get("kind") == "draft":
                handoff = {}  # the plan in the description is the plan: a draft gives way
            planned["handoff"] = {**handoff, "kind": "plan",
                                  "provider": handoff.get("provider") or record.get("planned_by")
                                  or "the issue's description", "notes": planned["plan_in_issue"]}
    set_state_label(ctx, number, names, LABEL_WORKING)
    if LABEL_NEEDS_PLAN in names and assignment.action != "plan":
        ctx.gh.remove_label(number, LABEL_NEEDS_PLAN)  # this run plans it first, in the run
    taken: list[str] = []
    def change(state: dict[str, Any]) -> None:
        entry = state_item(state, number)
        entry.update(run_id=cfg.run_id, started_at=iso(ctx.now()), kind=kind,
                     action=assignment.action, stop_requested=False, pending_request=False,
                     provider=provider.id)
        taken[:] = asks.take(entry)
        state["last_run"] = {"at": iso(ctx.now()), "url": cfg.run_url,
                             "what": f"{assignment.action} #{number} on {provider.id}"}
    ctx.store.update(change, f"claim #{number}")
    ctx.gh.create_comment(number, message)
    asks.react(ctx.gh, taken, asks.WORKING)
    return planned


def split_builders(mode: str) -> str:
    """Who builds a split's sub-issues (#60): this bot for `split` and `fullsend`, the other bot
    for `split-bot`."""
    if mode == "split-bot" and OTHERS:
        return f"{OTHERS[0].name} (@{OTHERS[0].login})"
    return "me"


def tree_text(ctx: Context, number: int) -> str:
    """The sub-issues an issue already has, for a split that closes its tree out: what was done
    and what is still open. Fenced as data; "" when it has none."""
    try:
        children = ctx.gh.list_sub_issues(number)
    except GitHubError:
        return ""
    if not children:
        return ""
    lines = [f"- #{c.get('number')} [{c.get('state')}] {c.get('title', '')}" for c in children]
    return data("\n".join(lines), f"The sub-issues #{number} has already")


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
