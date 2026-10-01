// Animate (docs/classic-sets.md B3.1, R383, R445): the verb an Animated Trap's or Field Trap's effect
// list ends with — "Then summon this as a Unit in Defense Position" (Classic #5 Tesla), "Summon this as
// a Unit" (Classic #38 Jackiestan Auctioneer). The move itself is `animated.animateCard`; this is the
// thin Effect a card script composes (CLAUDE.md rule 5).

import { animateCard } from "../animated";
import type { Effect } from "../script";
import type { Position } from "../state";
import { instanceOf, type TargetSpec } from "./targets";

export type AnimateArgs = {
  /** B3.1 rule 2: Attack Position unless the text says otherwise (Tesla: "DEF"). */
  position?: Position;
  /** The card to animate; the card running the script by default. */
  target?: TargetSpec;
};

/**
 * B3.1 rules 2 and 4: move the card from its backrow zone into its controller's unit zone in that lane,
 * else the leftmost open one, as a Unit — face-up, summoning sick, without leaving the field. A card
 * that is a Unit already stays where it is, in the position it has; with no open unit zone it stays in
 * the backrow (and a Trap stays there face-up, `traps.consumeTrap`). Not a summon (R445): it emits
 * `animated`, never `summoned`.
 */
export function animate(args: AnimateArgs = {}): Effect {
  return {
    kind: "animate",
    apply(ctx): void {
      const card = instanceOf(ctx, args.target ?? { of: "self" });
      if (card === null) return;
      animateCard(ctx, card, args.position === undefined ? {} : { position: args.position });
    },
  };
}
