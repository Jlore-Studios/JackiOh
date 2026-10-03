// Two small gifts a Classic+ card gives its own side (SPEC §8.7): Armor its hero keeps for the rest of
// the game (C+ #46 Felinor Flagbearer) and a discount on a random card of a hand (C+ #49 Jay Fungus).

import { effectiveCost, isXCost } from "../mana";
import type { Effect } from "../script";
import { setCostMod } from "./cost";
import { playerOf, type PlayerSpec } from "./targets";

/**
 * C+ #46: "Your hero gains +N Armor for the rest of the game" — §4.4 step 2's per-hit reduction, kept on
 * the hero itself (`hero.armor`), so it outlasts the card that gave it and stacks with every other
 * source (`damage.heroArmorOf`, R124). The view carries the hero's Armor; no event of its own.
 */
export function gainHeroArmor(args: { amount: number; player?: PlayerSpec }): Effect {
  return {
    kind: "gainHeroArmor",
    apply(ctx): void {
      const amount = Math.max(0, Math.trunc(args.amount));
      if (amount === 0) return;
      ctx.state.players[playerOf(ctx, args.player ?? "self")].hero.armor += amount;
    },
  };
}

/**
 * C+ #49: "A random card in your hand costs (N) less" — `costMod` −N on one card drawn uniformly from
 * the hand cards it can make cheaper: cost above 0 now (R65's `effectiveCost`) and not an X-cost card
 * (R65: modifiers never reach X). None such is nothing to do, so nothing is drawn (R129). The cost
 * floors at 0 when read (§2.3); `setCostMod` reports it under R177's sentinel to the other seat.
 */
export function discountRandomInHand(args: { amount: number; player?: PlayerSpec }): Effect {
  return {
    kind: "discountRandomInHand",
    apply(ctx): void {
      const hand = ctx.state.players[playerOf(ctx, args.player ?? "self")].hand;
      const cheaper = hand.filter((card) => !isXCost(ctx.state, card) && effectiveCost(ctx.state, card) > 0);
      if (cheaper.length === 0) return;
      const card = ctx.rng.pick(cheaper);
      if (card === undefined) return;
      setCostMod({ target: { of: "instance", instanceId: card.id }, amount: -Math.trunc(args.amount) }).apply(ctx);
    },
  };
}
