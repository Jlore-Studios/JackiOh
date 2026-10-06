// Random split damage (docs/classic-sets.md B5 E37): "deal N damage split among enemies".

import type { DamageTarget } from "../damage";
import { dealDamage } from "../damage";
import { unitView } from "../layers";
import type { Effect } from "../script";
import type { CardInstance, GameState } from "../state";
import { cardsInScope, playerOf } from "./targets";

/** "Still standing": above 0 health and not marked destroyed, so the check will not collect it (§4.5). */
function standing(state: GameState, unit: CardInstance): boolean {
  return unitView(state, unit).health > 0 && unit.markedDestroyed !== true;
}

/**
 * B5 E37: `amount` damage dealt as hits of `perHit` (1 unless the card says otherwise — Classic+ #3's
 * "1 damage for each Plague Counter", which an Upgrade may make 2), each hit a damage instance of its
 * own (§4.4) on an enemy drawn from the match rng among the ones still standing as that hit is dealt:
 * the enemy hero ("an enemy"; left out for "an enemy Unit") and every enemy unit acting on the field
 * still standing — above 0 health and not marked destroyed, since the state check does not run
 * between the hits of one effect (R59). A last hit smaller than `perHit` takes what is left. A Spell's split passes
 * a unit immune to Spells by, as every scope does (`effects/targets.ts`), and each hit is raised by
 * Spell Damage like any other hit of a Spell (E6).
 */
export function damageSplit(args: { amount: number; among?: "enemies" | "enemyUnits"; perHit?: number }): Effect {
  return {
    kind: "damageSplit",
    apply(ctx): void {
      const perHit = Math.max(1, Math.trunc(args.perHit ?? 1));
      const enemy = playerOf(ctx, "enemy");
      let left = Math.max(0, Math.trunc(args.amount));
      while (left > 0) {
        if (ctx.state.result !== null) return;
        const units = cardsInScope(ctx, { side: "enemy" }).filter((unit) => standing(ctx.state, unit));
        const pool: DamageTarget[] = [
          ...units.map((instance) => ({ kind: "unit" as const, instance })),
          ...(args.among === "enemyUnits" ? [] : [{ kind: "hero" as const, player: enemy }]),
        ];
        const target = ctx.rng.pick(pool);
        if (target === undefined) return;
        const hit = Math.min(perHit, left);
        dealDamage(ctx, { source: ctx.self, target, amount: hit });
        left -= hit;
      }
    },
  };
}
