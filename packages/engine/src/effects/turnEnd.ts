// Ending a turn from an effect (docs/classic-sets.md B5 E10, R456): Classic+ #26 Tommy Tempo's "End
// your turn" and its Radiant "You may take one more action. Then your turn ends", and the AI card Rate
// Limit's "After it resolves, their turn ends".
//
// Neither verb ends anything by itself. Each puts a "your turn ends" rider on the player whose turn it
// is (`modifiers.cutTurnShort`, a `turnEnds` modifier that lasts this turn), and `reduce.ts` ends the
// turn once it is due: the rest of the effect list resolves, then everything the action set off (the
// §10.3 loop), and only then does the turn end, as if its player had pressed End turn — every
// end-of-turn step runs (§2.2, R62), after `turnCutShort`. "One more action" counts that player's
// main-phase actions (a play, an attack, a position switch, an activation) and ends the turn once the
// last one has resolved; ending the turn themselves uses it up. On the other player's turn there is no
// turn of that player's to end, and nothing happens.

import { cutTurnShort } from "../modifiers";
import type { Effect } from "../script";
import { playerOf, type PlayerSpec } from "./targets";

/**
 * B5 E10, R456: `player`'s turn ends as soon as what is resolving now has resolved. "self" (the
 * default) is the controller's own turn (Tommy Tempo); "enemy" is the opponent's, whose play set a
 * trap off (Rate Limit: the play that set it off resolves first).
 */
export function endTurn(args: { player?: PlayerSpec } = {}): Effect {
  return {
    kind: "endTurn",
    apply(ctx): void {
      cutTurnShort(ctx, playerOf(ctx, args.player ?? "self"), 0, ctx.self?.id ?? null);
    },
  };
}

/**
 * B5 E10, R456: `player` may take `actions` more main-phase actions (default 1), then their turn ends
 * — once the last of them has resolved, prompts included. With a rider already on the turn, the
 * sooner end holds.
 */
export function endTurnAfterActions(args: { actions?: number; player?: PlayerSpec } = {}): Effect {
  return {
    kind: "endTurnAfterActions",
    apply(ctx): void {
      cutTurnShort(ctx, playerOf(ctx, args.player ?? "self"), Math.max(0, args.actions ?? 1), ctx.self?.id ?? null);
    },
  };
}
