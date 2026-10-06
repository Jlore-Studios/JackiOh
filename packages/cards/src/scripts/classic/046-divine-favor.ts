// C #46 Divine Favor (SPEC §8.6 row 46, BUILD M9 Classic row C 46). (1) Spell, Rare.
//   Base:    "Draw until you have as many cards in hand as your opponent." (1×, not tunable, R749)
//   Radiant: the same, at 2× — "draw until you have twice as many cards in hand as your opponent".
//   Engine:  "Read as it resolves, with this Spell already out of your hand: before each draw it
//            compares your hand with the opponent's (Radiant: with twice the opponent's) and draws one
//            card while yours is smaller; a draw that adds no card to your hand (a fatigue hit, a burn at
//            the hand cap, a card cast on draw, R58, or a draw the draw limit stops, §2.4) ends it, so it
//            can't loop; a hand already at the mark draws nothing. A `preview` (R280) shows how many
//            cards it would draw now. Tunes: multiplier 2 ↑, on the Radiant face only (R749)."
//
// The loop is the engine's `drawWhile`: the mark read before each draw, the first draw that adds no
// card ending it. The Spell is resolving, out of the hand (§10.5 step 4). `preview` (R280, proved in
// `test/preview.test.ts`): the draws it asks for now, from the two hands' public sizes, this card
// left out of "yours" in hand; label "Draw". `param(ctx, "multiplier")` (R386).

import type { ConditionContext, Script } from "@jackioh/engine";
import { param, zoneCount } from "@jackioh/engine";
import { drawWhile } from "@jackioh/engine/effects";
import { opponentOf, type PlayerId } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-046");

/** R280: the label the preview's value follows, an exact substring of both faces' text. */
const PREVIEW_LABEL = "Draw";

/**
 * The cards still wanted: `multiplier ×` the opponent's hand less your own, never below 0. `leaving`
 * counts the cards about to leave your hand before the draws begin (this card, asked in hand).
 */
function drawsWanted(state: ConditionContext["state"], player: PlayerId, multiplier: number, leaving = 0): number {
  const yours = zoneCount(state, player, "hand") - leaving;
  const mark = multiplier * zoneCount(state, opponentOf(player), "hand");
  return Math.max(0, mark - yours);
}

export const base: Script = {
  cry: () => [
    drawWhile({ more: (now) => drawsWanted(now.state, now.controller, param(now, "multiplier")) > 0 }),
  ],
  preview: (ctx) => {
    const inHand = ctx.zone === "hand" && ctx.self.zone.z === "hand" ? 1 : 0;
    return [{ label: PREVIEW_LABEL, value: drawsWanted(ctx.state, ctx.controller, param(ctx, "multiplier"), inHand) }];
  },
};

// The same script: the Radiant face differs only in its declared multiplier (2), read through `param`.
export const radiant: Script = base;
