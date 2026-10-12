"""What the queue waits for, and what each subscription can do now: the status issue's Queue and
its Subscriptions (#148).

Worked out with the planner's own rules, never a copy of them: the queue is `queue.candidates`,
and who takes what is `plan.pairs` run on the lanes held now, one item at a time as the night
runs claim them (`plan.fill_lanes`). So the issue says an item starts next only when the planner
would start it, and calls a subscription idle only when it may start work and nothing queued is
for it. An item no run can take says the tier its difficulty needs (an Opus for a hard one, or a
revision that answers a strong audit), which subscriptions have such a seat, and when the first
of them can start one: at a reset, at its hours, after its back-off, or when its run ends.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime
from typing import Any

from harness import plan as plan_mod
from harness import providers as providers_mod
from harness.clock import parse_iso, zone
from harness.config import DIFFICULTIES, MIN_TIER, PART_FLOOR
from harness.context import Context
from harness.providers import TIER_RANK, Pool, Provider
from harness.queue import (KIND_ORDER, PRIORITY_NAMES, Candidate, candidates, needs_plan,
                           plan_floor, plan_meets)

#: What each tier may take, said once above the queue: the router tries the weakest that may.
TIER_WORDS = {"weak": "any model", "medium": "a medium or strong model", "strong": "a strong model"}
DIFFICULTY_MARK = {"easy": "🟢", "medium": "🟡", "hard": "🔴", "unrated": "⚪"}


@dataclass
class Now:
    """One subscription now: `kind` is busy, next (the planner gives it an item at the next
    run), idle (it may start work, and nothing queued is for it), reviews (capped for long runs,
    but under the cap a review goes up to), full (no lane free where it runs), capped, closed
    (outside its hours), broken (backing off after a run that could not work) or off. `until` is
    when it can start a build or a plan again, when that is known."""

    kind: str
    text: str
    until: datetime | None = None
    items: list[int] = field(default_factory=list)


@dataclass
class Row:
    """One queued item, and what it waits for."""

    number: int
    title: str
    #: What the next run does to it: plan, build, revise or review.
    step: str
    #: Its difficulty, or `unrated` (routed as medium until its planner rates it).
    difficulty: str
    #: The weakest tier that may take that step, and why it is not the difficulty's own.
    tier: str
    tier_why: str
    priority: str
    #: Who takes it at the next run (`plan.Assignment`), or None.
    starts: Any = None
    #: Why nothing takes it now, in a line.
    waiting: str = ""
    #: True for an item the planner leaves out of the queue (`human`, a blocker, open parts).
    held: bool = False
    forced: bool = False


@dataclass
class Explained:
    rows: list[Row]
    #: Each subscription now, by id.
    now: dict[str, Now]
    halted: str = ""


def tier_names(pool: Pool, tier: str) -> str:
    """The models of `tier`, short: "Opus", or "Muse, agy (Gemini), Sonnet"."""
    names = []
    for entry in pool.tiers.get(tier, ()):
        owners = [p for p in pool.ordered()
                  if any(s.model == entry.model for s in pool.own_seats(p))]
        if not owners:
            continue
        if all(p.cli == "claude" for p in owners):
            names.append(entry.model.capitalize())
        else:
            names.append(", ".join(dict.fromkeys(p.id for p in owners)))
    return ", ".join(dict.fromkeys(names)) or "none"


def lanes_of(ctx: Context, state: dict[str, Any], live: dict[int, str]) -> plan_mod.Lanes:
    """`plan.read_lanes` from the lanes the issue has read already (`status.lanes_now`), so the
    queue costs no second look at the runs."""
    pool = ctx.cfg.pool
    planning = {n for n in live
                if n and (state["items"].get(str(n)) or {}).get("action") == "plan"}
    return plan_mod.Lanes(pool.max_parallel, dict(live), planning, pool.plan_lanes,
                          pool.machine_parallel,
                          frozenset(p.id for p in pool.ordered() if pool.on_machine(p.id)))


def need(c: Candidate) -> tuple[str, str, str]:
    """(step, tier, why): what the next run does to `c`, the weakest tier that may do it
    (`plan.assign`, `plan.review_seat`), and why that tier is not its difficulty's own."""
    if c.kind == "review":
        tier, why = ("medium", "") if c.difficulty == "easy" else ("strong", "its final audit")
        if c.review_floor in TIER_RANK and TIER_RANK[c.review_floor] > TIER_RANK[tier]:
            tier, why = c.review_floor, f"a person asked for a {c.review_floor} review"
        return "review", tier, why
    if needs_plan(c):
        why = "rates and plans it first" if not c.rated else "a plan first"
        return "plan", plan_floor(c), why
    tier, why = MIN_TIER[c.difficulty], ""
    if c.kind == "build" and c.part and plan_meets(c) and TIER_RANK[tier] > TIER_RANK[PART_FLOOR]:
        tier, why = PART_FLOOR, "a fullsend part, built from its plan"
    if c.kind == "revise":
        if c.audit_tier in TIER_RANK and TIER_RANK[c.audit_tier] > TIER_RANK[tier]:
            tier, why = c.audit_tier, f"it answers a {c.audit_tier} reviewer's audit"
        if (not c.bot_pr or c.conflict) and TIER_RANK[tier] < TIER_RANK["medium"]:
            tier = "medium"
            why = ("a conflict with main wants a reviewer in its run" if c.conflict
                   else "a person's pull request wants a reviewer in its run")
    return c.kind, tier, why


def _clock(ctx: Context, at: datetime) -> str:
    """`20:27` today (Central time), `Fri 18:47` on another day."""
    local = at.astimezone(zone(ctx.cfg.timezone))
    today = ctx.now().astimezone(zone(ctx.cfg.timezone))
    return local.strftime("%H:%M" if local.date() == today.date() else "%a %H:%M")


def held_by(ctx: Context, state: dict[str, Any], provider: Provider, *, forced: bool = False,
            starting: bool = True) -> Now | None:
    """Why `provider` may not start a run now, as a `Now`, or None when it may (its lanes
    aside). In the order `providers.availability` checks."""
    cfg, now = ctx.cfg, ctx.now()
    reason = providers_mod.availability(provider, state, now, cfg.timezone, cfg.secrets,
                                        forced=forced, starting=starting)
    if reason is None:
        return None
    if not provider.enabled or providers_mod.switched_off_by_date(provider, now, cfg.timezone):
        return Now("off", "⚫ switched off")
    if providers_mod.suspension(state, provider.id) is not None:
        return Now("off", "⚫ suspended")
    if provider.login == "secret" and cfg.secrets.has(provider.secret) is False:
        return Now("off", "⚫ off: no secret set")
    window = provider.schedule.window(cfg.timezone)
    if (not forced and not provider.off_hours and window is not None
            and not window.is_open(now)):
        opens = window.next_open(now)
        return Now("closed", f"🌙 outside its hours<br/>opens {_clock(ctx, opens)}", opens)
    entry = providers_mod.peek_record(state, provider.id)
    infra = entry.get("infra") if isinstance(entry.get("infra"), dict) else {}
    failed = parse_iso(infra.get("at"))
    if not forced and failed is not None and now - failed < providers_mod.infra_backoff(infra):
        back = failed + providers_mod.infra_backoff(infra)
        return Now("broken", f"⛔ its last run could not work<br/>tries again {_clock(ctx, back)}",
                   back)
    frees = providers_mod.frees_at(provider, entry, now, cfg.timezone)
    # Under its cap but too close to it for a long run: a review, which goes up to the cap, may
    # still start (`plan._usable`).
    reviews = (starting and "review" in provider.roles and providers_mod.availability(
        provider, state, now, cfg.timezone, cfg.secrets, forced=forced) is None)
    return Now("reviews" if reviews else "capped",
               "⏸️ " + capped_text(ctx, provider, entry, frees, reviews=reviews), frees)


def capped_text(ctx: Context, provider: Provider, entry: dict[str, Any],
                frees: datetime | None, *, reviews: bool = False) -> str:
    """Which limit holds a capped subscription, and when it can start a build or a plan again."""
    now = ctx.now()
    until = parse_iso(entry.get("refused_until"))
    if until is not None and until > now:
        return f"refused a call<br/>until {_clock(ctx, until)}"
    usage = providers_mod.current_usage(provider, entry)
    again = f"<br/>starts again {_clock(ctx, frees)}" if frees else ""
    for window, cap in provider.caps_at(now, ctx.cfg.timezone).items():
        reading = usage.get(window) if isinstance(usage, dict) else None
        if not isinstance(reading, dict):
            continue
        resets = parse_iso(reading.get("resets_at"))
        if resets is not None and resets <= now:
            continue
        used = float(reading.get("utilization") or 0)
        start = providers_mod.start_under(provider, window, cap)
        if used < start:
            continue
        held = f"{providers_mod.WINDOW_NAMES[window]} {used:.0%} of its {cap:.0%} cap"
        if used < cap:
            held += f"<br/>long runs start under {start:.0%}" + ("; reviews only" if reviews else "")
        return held + again
    for window, budget in provider.limits.budgets.items():
        spent = providers_mod.minutes_spent(entry, now - providers_mod.WINDOWS[window])
        if spent >= budget:
            return f"{spent:.0f} of its {budget} min" + again
    return "at its limit" + again


def explain(ctx: Context, state: dict[str, Any], live: dict[int, str], *,
            issues: list[dict[str, Any]] = (), halted: str = "") -> Explained:
    """The queue in the planner's order, each item with what takes it next or what it waits
    for, and each subscription now."""
    held_back: dict[int, str] = {}
    queue = candidates(ctx, state, held_back=held_back)
    lanes = lanes_of(ctx, state, live)
    starts: dict[int, Any] = {}
    after = lanes
    if not halted:
        rest = list(queue)
        for _ in range(len(queue)):
            top = next(plan_mod.pairs(ctx, state, rest, after, force=False,
                                      quiet_ok=plan_mod.ANY_QUIET), None)
            if top is None:
                break
            candidate, assignment = top
            starts[candidate.number] = assignment
            after = after.with_run(candidate.number, assignment.provider.id, assignment.action)
            rest = [c for c in rest if c.number != candidate.number]
    now = subscriptions(ctx, state, lanes, after, starts, halted=halted)
    # The planner's own order (`plan.pairs`), the items that start at the next run first.
    order = sorted(queue, key=lambda c: (c.number not in starts, not c.forced, c.kind != "review",
                                         c.priority, KIND_ORDER[c.kind],
                                         -DIFFICULTIES.index(c.difficulty), c.queued_at,
                                         c.number))
    rows = []
    for c in order:
        step, tier, why = need(c)
        difficulty = c.difficulty if c.rated else "unrated"
        row = Row(c.number, c.title, step, difficulty, tier, why,
                  PRIORITY_NAMES.get(c.priority, "none"), starts.get(c.number), forced=c.forced)
        if row.starts is None:
            row.waiting = halted or waits_for(ctx, state, c, step, tier, after, now)
        rows.append(row)
    rows += held_rows(held_back, issues)
    return Explained(rows, now, halted)


def held_rows(held_back: dict[int, str], issues: Any) -> list[Row]:
    """The labelled items the planner leaves out, each with why."""
    titles = {int(t["number"]): str(t.get("title") or "") for t in issues}
    rows = []
    for number, why in sorted(held_back.items()):
        rows.append(Row(number, titles.get(number, ""), "build", "", "", "", "none",
                        waiting=why, held=True))
    return rows


def subscriptions(ctx: Context, state: dict[str, Any], lanes: plan_mod.Lanes,
                  after: plan_mod.Lanes, starts: dict[int, Any], *, halted: str = ""
                  ) -> dict[str, Now]:
    """Each subscription now. Busy is a run going; next is an item the planner gives it at the
    next run; idle is free to start and nothing queued is for it. One that may start but has no
    lane where it runs is full, never free."""
    pool = ctx.cfg.pool
    nexts: dict[str, list[int]] = {}
    for number, assignment in starts.items():
        nexts.setdefault(assignment.provider.id, []).append(number)
    found: dict[str, Now] = {}
    for provider in pool.ordered():
        running = sorted(n for n, p in lanes.held.items() if p == provider.id)
        if provider.id in nexts:
            found[provider.id] = Now("next", "▶️ takes " + ", ".join(
                f"#{n}" for n in nexts[provider.id]) + " next", items=nexts[provider.id])
            if running:
                found[provider.id].items = running + nexts[provider.id]
            continue
        if running:
            found[provider.id] = Now("busy", "🟢", items=running)
            continue
        held = held_by(ctx, state, provider)
        if held is not None:
            found[provider.id] = held
            continue
        if halted:
            found[provider.id] = Now("off", "🛑 halted")
        elif plan_mod.machine_full(pool, provider, lanes):
            found[provider.id] = Now("full", f"⏸️ the night box is full<br/>"
                                     f"({pool.machine_parallel} at once)")
        elif not lanes.room_for(provider.id) and not lanes.plan_free:
            found[provider.id] = Now("full", f"⏸️ every lane is in use<br/>"
                                     f"({pool.max_parallel} on GitHub's runners)")
        else:
            found[provider.id] = Now("idle", "⚪ idle<br/>nothing queued is for it")
    return found


def waits_for(ctx: Context, state: dict[str, Any], c: Candidate, step: str, tier: str,
              lanes: plan_mod.Lanes, now: dict[str, Now]) -> str:
    """Why no run takes `c` now: the kind of model it needs, and which of them frees up first."""
    pool = ctx.cfg.pool
    role = plan_mod.ROLE_OF.get(step, step)
    able = [p for p in pool.ordered()
            if role in p.roles and pool.best_seat(p, tier) is not None]
    if not able:
        return f"⚠️ no subscription has a {tier} seat that may {role} it"
    wanted = (f"a {'medium or strong' if tier == 'medium' else tier} planner" if step == "plan"
              else TIER_WORDS[tier])
    if tier == "strong":
        wanted += f" ({tier_names(pool, tier)})"
    soonest: tuple[datetime, str] | None = None
    busy, taking, idle = [], [], []
    for provider in able:
        held = now.get(provider.id)
        if held is None:
            continue
        if held.kind == "busy":
            busy.append(f"`{provider.id}`")
        elif held.kind == "next":
            others = ", ".join(f"#{n}" for n in held.items if n != c.number)
            taking.append(f"`{provider.id}` takes {others} first")
        elif held.until is not None:
            if soonest is None or held.until < soonest[0]:
                soonest = (held.until, provider.id)
        elif held.kind in ("idle", "full"):
            idle.append(provider.id)
    parts = []
    if soonest is not None:
        parts.append(f"first free `{soonest[1]}` at {_clock(ctx, soonest[0])}")
    if busy:
        parts.append(f"busy {', '.join(busy)}")
    parts += taking
    if not parts and idle:
        full = [p for p in idle if now[p].kind == "full"]
        parts.append("no lane is free where they run" if full
                     else f"{', '.join(f'`{p}`' for p in idle)} free, but the planner's rules "
                     "pass them over")
    return f"⏳ **{wanted}**" + (": " + " · ".join(parts) if parts else "")
