// Two random picks of existing cards that Classic #90 In Too Deep's rewards name and no verb had
// (SPEC §8.6 row 90; R60): "a random Unit of yours gets +3/+3" (reward E) and "return 2 random cards
// from your graveyard to your hand" (reward C). A hook may not roll dice — `ctx.rng` advances
// `rngCursor`, which is state — so the pick happens here, as the effect applies.

import { addToHand as putInHand } from "../draw";
import type { Effect } from "../script";
import { activeUnitsOf } from "../zones";
import { buff, type BuffAmount } from "./buff";
import { playerOf, type PlayerSpec } from "./targets";

/**
 * "A random Unit of yours gets +X/+Y": one Unit acting on that side (the tops of the piles, R13),
 * drawn with the match rng, gets a layer-4 buff (§10.4) through `buff`. No Unit draws nothing (R129).
 */
export function buffRandomUnit(args: { player?: PlayerSpec } & BuffAmount): Effect {
  return {
    kind: "buffRandomUnit",
    apply(ctx): void {
      const units = activeUnitsOf(ctx.state, playerOf(ctx, args.player ?? "self"));
      if (units.length === 0) return;
      const unit = ctx.rng.pick(units);
      if (unit === undefined) return;
      const amount: BuffAmount = {
        ...(args.attack === undefined ? {} : { attack: args.attack }),
        ...(args.health === undefined ? {} : { health: args.health }),
      };
      buff({ target: { of: "instance", instanceId: unit.id }, ...amount }).apply(ctx);
    },
  };
}

/**
 * "Return N random cards from your graveyard to your hand": R60's random pick of N existing cards —
 * N different cards, or all of them if fewer lie there — drawn together before any moves, so a card a
 * full hand burns back into the graveyard (§2.4, R4) is not picked again. Each goes to its owner's
 * hand through §2.4's pipeline in the order drawn. An empty graveyard draws nothing (R129).
 */
export function returnRandomFromGraveyard(args: { count: number; player?: PlayerSpec }): Effect {
  return {
    kind: "returnRandomFromGraveyard",
    apply(ctx): void {
      const graveyard = ctx.state.players[playerOf(ctx, args.player ?? "self")].graveyard;
      const count = Math.max(0, Math.trunc(args.count));
      if (count === 0 || graveyard.length === 0) return;
      for (const card of ctx.rng.shuffle(graveyard).slice(0, count)) putInHand(ctx, card);
    },
  };
}
