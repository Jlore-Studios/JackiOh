// The Lock variants and Unlock (docs/classic-sets.md B5 E20; SPEC §3.2 Lock). A Lock lives on the zone
// and outlives every occupant: it evicts nothing, and the zone takes no summon, play or return until an
// Unlock opens it again. `effects/counters.ts` keeps the single `lock` and `unlock`; these are the
// forms the new cards name — a whole lane (Classic #71 Lane Eater), the zone a permanent was just
// played into (Classic #84 Lockdown, Classic+ #34 Memory Leak), a random zone not already Locked
// (Classic+ #34), the firing trap's own zone (Classic+ #1 Doom Shroom) and every zone (Classic+ #77
// Anti-Softlock).

import type { GameEvent, Row } from "@jackioh/shared";
import type { Effect, EffectContext } from "../script";
import { findInstance } from "../state";
import { eventMark, leftFieldAfter } from "../stays";
import { isLocked, lockZone, rowSize, slotOf, slotsOf, unlockZone, type ZoneSlot } from "../zones";
import { EVENT_KEY } from "../work";
import { instanceOf, sidesOf, type TargetSpec } from "./targets";

/** Which sides and rows a zone-wide Lock or Unlock covers: both sides and both rows by default. */
export type ZoneScope = { side?: "any" | "self" | "enemy"; rows?: Row[] };

const BOTH_ROWS: readonly Row[] = ["units", "backrow"];

/** The zones a scope covers, in R68's walk: side by side, units then backrow, lane 1 upward. */
function zonesInScope(ctx: EffectContext, scope: ZoneScope): ZoneSlot[] {
  const rows = scope.rows ?? BOTH_ROWS;
  return sidesOf(ctx, scope.side).flatMap((player) => rows.flatMap((row) => slotsOf(player, row)));
}

function lockOne(ctx: EffectContext, ref: ZoneSlot): void {
  if (isLocked(ctx.state, ref)) return;
  lockZone(ctx.state, ref);
  ctx.events.push({ type: "locked", player: ref.player, row: ref.row, lane: ref.lane });
}

function unlockOne(ctx: EffectContext, ref: ZoneSlot): void {
  if (!isLocked(ctx.state, ref)) return;
  unlockZone(ctx.state, ref);
  ctx.events.push({ type: "unlocked", player: ref.player, row: ref.row, lane: ref.lane });
}

/** Which lane: the one a card stands in (`self` by default, or a named card), or a number. */
export type LaneSpec = number | TargetSpec;

function laneOf(ctx: EffectContext, spec: LaneSpec): number | null {
  if (typeof spec === "number") return spec >= 1 && spec <= rowSize("units") ? spec : null;
  const card = spec.of === "self" ? ctx.self : instanceOf(ctx, spec);
  const at = card === null ? null : slotOf(ctx.state, card);
  return at?.lane ?? null;
}

/**
 * B5 E20, §3.1: Lock a whole lane — every zone of it the scope covers, both sides' unit and backrow
 * zones by default (Classic #71's "Lock this lane"; its Radiant "the enemy side of this lane" is
 * `side: "enemy"`). "This lane" is the lane the card running the script stands in. A zone Locked
 * already stays as it is; a Lock evicts nothing, so the card that locked its own zone stays in it.
 */
export function lockLane(args: { lane?: LaneSpec } & ZoneScope = {}): Effect {
  return {
    kind: "lockLane",
    apply(ctx): void {
      const lane = laneOf(ctx, args.lane ?? { of: "self" });
      if (lane === null) return;
      for (const ref of zonesInScope(ctx, args)) if (ref.lane === lane) lockOne(ctx, ref);
    },
  };
}

/**
 * The zone the permanent an arrival event names was just put into: a `summoned` event names it; a
 * `cardPlayed` names the card, whose zone is where it stands, if it still stands there on the stay the
 * play put it on (R174, R212). A Spell's play puts nothing anywhere.
 */
function zonePlayedInto(ctx: EffectContext, event: GameEvent): ZoneSlot | null {
  if (event.type === "summoned") return { player: event.player, row: event.row, lane: event.lane };
  if (event.type !== "cardPlayed" && event.type !== "cardResolved") return null;
  const card = findInstance(ctx.state, event.instanceId);
  if (card === undefined || card.zone.z !== "field") return null;
  const mark = eventMark(event);
  if (mark !== undefined && leftFieldAfter(ctx.state, mark, card.id)) return null;
  return slotOf(ctx.state, card);
}

/**
 * B5 E20: Lock the zone a permanent was just played into (Classic #84 Lockdown's "After a permanent is
 * played, Lock its zone", Classic+ #34). `event` is the trigger's own (`ctx.event`, read from the
 * context's captured event when omitted): a `summoned`, a `cardPlayed` or a `cardResolved`. A played
 * Spell, or a permanent already gone from where it landed, locks nothing.
 */
export function lockPlayedZone(args: { event?: GameEvent } = {}): Effect {
  return {
    kind: "lockPlayedZone",
    apply(ctx): void {
      const event = args.event ?? (ctx.data[EVENT_KEY] as GameEvent | undefined);
      if (event === undefined || event === null || typeof event !== "object") return;
      const ref = zonePlayedInto(ctx, event);
      if (ref !== null) lockOne(ctx, ref);
    },
  };
}

/**
 * B5 E20: Lock one random zone the scope covers that is not Locked already — an occupied one is fine,
 * a Lock evicts nothing (Classic+ #34 Memory Leak: "a random zone on your opponent's side", `side:
 * "enemy"`). One uniform draw from the match rng among the candidates (R60); none left, nothing.
 */
export function lockRandomZone(args: ZoneScope = {}): Effect {
  return {
    kind: "lockRandomZone",
    apply(ctx): void {
      const open = zonesInScope(ctx, args).filter((ref) => !isLocked(ctx.state, ref));
      if (open.length === 0) return;
      const ref = open[ctx.rng.int(open.length)];
      if (ref !== undefined) lockOne(ctx, ref);
    },
  };
}

/**
 * B5 E20: Lock the zone the card running the script stands in — a firing trap's own backrow zone
 * (Classic+ #1 Doom Shroom's "Lock this zone"). The trap is still in it while its list runs, and is
 * consumed after (`traps.consumeTrap`), leaving the zone Locked behind it. A card its own list moved off
 * the field first locks nothing.
 */
export function lockOwnZone(): Effect {
  return {
    kind: "lockOwnZone",
    apply(ctx): void {
      const self = ctx.self;
      const ref = self === null ? null : slotOf(ctx.state, self);
      if (ref !== null) lockOne(ctx, ref);
    },
  };
}

/**
 * B5 E20: Unlock every Locked zone the scope covers — both sides and both rows by default (Classic+
 * #77 Anti-Softlock's "Unlock every zone") — one `unlocked` event per zone opened, in R68's walk.
 */
export function unlockAll(args: ZoneScope = {}): Effect {
  return {
    kind: "unlockAll",
    apply(ctx): void {
      for (const ref of zonesInScope(ctx, args)) unlockOne(ctx, ref);
    },
  };
}
