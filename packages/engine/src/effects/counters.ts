// Counters on an instance and locks on a zone (§6.3). Plague Counters live on the instance and R78
// clears them when the card leaves the field; a Lock lives on the zone and outlives every occupant.
// The Plague Counter rules themselves — what a placement is, the multiplier, the report — are
// `../plague`'s (R471); the placement verbs of patch v0.2.0 are `./plague`'s.

import type { Row } from "@jackioh/shared";
import { placePlagueOn, plagueOn, removePlague } from "../plague";
import type { Effect, EffectContext } from "../script";
import type { CardInstance } from "../state";
import { isLocked, lockZone, rowSize, slotOf, unlockZone, type ZoneSlot } from "../zones";
import { playerOf, type PlayerSpec, resolveTarget, type TargetSpec } from "./targets";

function instanceOf(ctx: EffectContext, spec: TargetSpec): CardInstance | null {
  const target = resolveTarget(ctx, spec);
  return target === null || target.kind !== "unit" ? null : target.instance;
}

/**
 * #91 Fed Fauci: add Plague Counters to a permanent, any number of them — one placement (R471, so a
 * card that multiplies what is placed on it multiplies this, and "whenever Plague Counters are placed
 * on this" answers it). A negative amount takes them off and the count floors at 0; R78 resets the
 * counter when the card leaves the field.
 */
export function plague(args: { target?: TargetSpec; amount: number }): Effect {
  return {
    kind: "plague",
    apply(ctx): void {
      const card = instanceOf(ctx, args.target ?? { of: "self" });
      if (card === null) return;
      const amount = Math.trunc(args.amount);
      if (amount > 0) placePlagueOn(ctx, card, amount);
      else removePlague(ctx, card, -amount);
    },
  };
}

/** Clear every Plague Counter on a permanent. */
export function clearPlague(args: { target?: TargetSpec } = {}): Effect {
  return {
    kind: "clearPlague",
    apply(ctx): void {
      const card = instanceOf(ctx, args.target ?? { of: "self" });
      if (card === null) return;
      removePlague(ctx, card, plagueOn(card));
    },
  };
}

/**
 * Which zone a Lock names: the one this card sits in, the one a named card sits in (#36 Magic
 * Jammed locks its target's zone, so the lock effect runs before the destroy that empties it), or
 * a lane by index (§3.1 "this lane").
 */
export type ZoneSpec =
  | { of: "self" }
  | { of: "chosen"; index?: number }
  | { of: "lane"; row: Row; lane: number; player?: PlayerSpec };

function zoneFor(ctx: EffectContext, spec: ZoneSpec): ZoneSlot | null {
  if (spec.of === "lane") {
    if (spec.lane < 1 || spec.lane > rowSize(spec.row)) return null;
    return { player: playerOf(ctx, spec.player ?? "self"), row: spec.row, lane: spec.lane };
  }
  const card = spec.of === "self" ? ctx.self : instanceOf(ctx, spec);
  return card === null ? null : slotOf(ctx.state, card);
}

/**
 * §3.2 Lock: the zone accepts no summons until something unlocks it. The current occupant is
 * unaffected and the lock persists after it leaves. Nothing in Core unlocks a zone; B5 E20's Unlock
 * (`unlock` below, `effects/locks.unlockAll`) does.
 */
export function lock(args: { zone: ZoneSpec }): Effect {
  return {
    kind: "lock",
    apply(ctx): void {
      const ref = zoneFor(ctx, args.zone);
      if (ref === null || isLocked(ctx.state, ref)) return;
      lockZone(ctx.state, ref);
      ctx.events.push({ type: "locked", player: ref.player, row: ref.row, lane: ref.lane });
    },
  };
}

/**
 * B5 E20: a Locked zone accepts summons again (event `unlocked`). A zone that is not Locked is left
 * as it is, with no event. Its occupant is unaffected, and a card whose return a Lock stopped (an
 * animated card's home, B3.1 rule 6) goes back at its next chance.
 */
export function unlock(args: { zone: ZoneSpec }): Effect {
  return {
    kind: "unlock",
    apply(ctx): void {
      const ref = zoneFor(ctx, args.zone);
      if (ref === null || !isLocked(ctx.state, ref)) return;
      unlockZone(ctx.state, ref);
      ctx.events.push({ type: "unlocked", player: ref.player, row: ref.row, lane: ref.lane });
    },
  };
}
