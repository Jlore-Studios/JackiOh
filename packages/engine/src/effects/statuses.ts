// Unit statuses an effect sets (docs/classic-sets.md B5 E35): Berserk, and "may attack again".
//
// A status is not text, so neither is a keyword: a Vanilla keeps it, and R78's reset takes it off
// with the unit leaving the field. What a Berserk unit does about it — Classic+ #19.5's forced
// attacks on its own hero at the start and end of its controller's turn — is its own card's text,
// written with `forcedAttackOwnHero` and read through `isBerserk` (`restrictions.ts`).

import { BERSERK_MARK } from "../config";
import { canGoBerserk } from "../restrictions";
import type { Effect } from "../script";
import { instanceOf, type TargetSpec } from "./targets";

/**
 * B5 E35: the unit goes Berserk (Classic+ #19.2's reward for Classic+ #19.5). Nothing happens to a
 * unit that is Berserk already, is not on the field, or "can't go Berserk" (`neverBerserk`, #19.5's
 * Radiant face). Both views show it (`UnitView.berserk`), and R437's reusable mark announces it:
 * `marked`, "berserk", red.
 */
export function goBerserk(args: { target: TargetSpec }): Effect {
  return {
    kind: "goBerserk",
    apply(ctx): void {
      const unit = instanceOf(ctx, args.target);
      if (unit === null || !canGoBerserk(ctx.state, unit)) return;
      unit.berserk = true;
      ctx.events.push({ type: "marked", instanceId: unit.id, mark: BERSERK_MARK.mark, color: BERSERK_MARK.color, added: true });
    },
  };
}

/**
 * B5 E35: the unit may attack again this turn (Classic+ #73.1 Classic Golem's "after it destroys a
 * Unit"): a fresh exertion — both halves, §4.1 — and no summoning sickness for the rest of this turn,
 * so a unit that has just entered the field may attack at once. It does not grant an attack a unit
 * could not otherwise make: Attack Position, attack above 0 and the unit restrictions still apply.
 */
export function mayAttackAgain(args: { target: TargetSpec }): Effect {
  return {
    kind: "mayAttackAgain",
    apply(ctx): void {
      const unit = instanceOf(ctx, args.target);
      if (unit === null || unit.zone.z !== "field" || unit.zone.row !== "units") return;
      unit.exertion = { attacked: false, switched: false };
      // §4.1: sickness is "entered the field this turn" (`combat.isSick`), which this lifts for the turn.
      if (unit.summonedTurn === ctx.state.turn) delete unit.summonedTurn;
    },
  };
}
