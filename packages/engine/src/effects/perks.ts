// Small gifts a card gives its own side: Armor its hero keeps for the rest of the game (C+ #46 Felinor
// Flagbearer, SPEC §8.7) or until its next turn (Core #98's Armor Up, patch v0.2.1, R603), and a discount
// on a random card of a hand (C+ #49 Jay Fungus).

import { effectiveCost, isXCost } from "../mana";
import { addModifier } from "../modifiers";
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
 * R603, Core #98's Armor Up: "Your hero gains N Armor until your next turn" — the same per-hit
 * reduction, held as a `heroArmor` modifier on the player that ends as that player's next turn starts
 * (`modifiers.expireAtTurnStart`), so it covers the opponent's turn between and never touches the
 * Armor written on the hero. Its badge is the modifier's (R169).
 */
export function gainHeroArmorUntilNextTurn(args: { amount: number; player?: PlayerSpec }): Effect {
  return {
    kind: "gainHeroArmorUntilNextTurn",
    apply(ctx): void {
      const amount = Math.max(0, Math.trunc(args.amount));
      if (amount === 0) return;
      const player = playerOf(ctx, args.player ?? "self");
      addModifier(ctx, player, {
        kind: "heroArmor",
        amount,
        expiry: { until: "startOfTurnOf", player, fromTurn: ctx.state.turn },
      });
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
