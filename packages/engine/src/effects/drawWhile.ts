// "Draw until …" (Classic #46 Divine Favor, SPEC §8.6 row 46): one draw at a time while a condition the
// card reads holds, asked again before each draw, ending at the first draw that adds no card to the
// hand — a fatigue hit, a burn at the hand cap, a card cast on draw (R58), a draw a draw limit stops
// (§2.4) — so it never loops. A card-specific verb of the cards-classic-b workstream.
//
// Each draw is §2.4's own (`draw.draw`). A cast on draw ends it whatever its chain then draws, and a
// cast that asks a question leaves the rest of its chain owed to the answer (R113) and ends it too.
// The hand holds at most `HAND_CAP` cards, which bounds the loop.

import { HAND_CAP } from "../config";
import { draw as drawCards } from "../draw";
import type { Effect, EffectContext } from "../script";
import { playerOf, type PlayerSpec } from "./targets";

/** Draw one card at a time while `more(ctx)` holds, stopping at a draw that adds no card to the hand. */
export function drawWhile(args: { more: (ctx: EffectContext) => boolean; player?: PlayerSpec }): Effect {
  return {
    kind: "drawWhile",
    apply(ctx): void {
      const player = playerOf(ctx, args.player ?? "self");
      const asked = ctx.state.pending;
      for (let draws = 0; draws <= HAND_CAP; draws += 1) {
        if (ctx.state.result !== null || !args.more(ctx)) return;
        const held = ctx.state.players[player].hand.length;
        const [outcome] = drawCards(ctx, player, 1);
        if (ctx.state.pending !== asked) return;
        if ((outcome !== "drawn" && outcome !== "token") || ctx.state.players[player].hand.length <= held) return;
      }
    },
  };
}
