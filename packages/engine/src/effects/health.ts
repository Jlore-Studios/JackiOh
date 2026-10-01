// Set health, and heals turned into damage (docs/classic-sets.md B5 E7, E8).

import { setHeroHealth } from "../damage";
import { addModifier } from "../modifiers";
import type { Effect } from "../script";
import { playerOf, resolveTarget, type PlayerSpec, type TargetSpec } from "./targets";

/**
 * B5 E7: "Set a hero's health to N" (Classic #29 Book of Vital Kill, either hero, a declared target).
 * No pipeline, not damage and not a heal: no Armor, no cap, no replacement, nothing that answers a hit
 * or a heal (R18's lose health is the nearest rule). A target that is not a hero fizzles. `healthSet`.
 */
export function setHealth(args: { to: TargetSpec; value: number }): Effect {
  return {
    kind: "setHealth",
    apply(ctx): void {
      const target = resolveTarget(ctx, args.to);
      if (target === null || target.kind !== "hero") return;
      setHeroHealth(ctx, target.player, args.value, ctx.self?.id ?? null);
    },
  };
}

/**
 * B5 E8: "for the rest of this turn, healing on enemies deals that much Pierce damage to them
 * instead" — a this-turn player modifier on `player` (default "self"), converting every heal on that
 * player's enemies from then on, the damage from this card. Classic+ #22 Blood Moon's base face does
 * the same through its replacement's `lasting: "thisTurn"`, which also converts the heal that set it
 * off; this verb is the plain effect for a card that says it outright.
 */
export function convertHealing(args: { player?: PlayerSpec } = {}): Effect {
  return {
    kind: "convertHealing",
    apply(ctx): void {
      const self = ctx.self;
      if (self === null) return;
      addModifier(ctx, playerOf(ctx, args.player ?? "self"), {
        kind: "healToDamage",
        converterId: self.id,
        expiry: { until: "thisTurn", turn: ctx.state.turn },
      });
    },
  };
}
