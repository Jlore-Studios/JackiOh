// C #26 Rapid Draw (SPEC §8.6 row 26). (0) Spell, Common.
//   Base:    "Draw {draw}. Then discard {discard|card|cards}." — draw 4, discard 4
//   Radiant: "Draw {draw}. Then discard {discard|card|cards}." — draw 5, discard 4
//   Engine:  "Four draws (Radiant five; the hand cap applies, R4), then 4 random discards (R641;
//            fewer in hand, all of them). Tunes: draw 4 ↑; discard 4 ↓."
//
// The draws are §2.4's pipeline (`draw`): each its own cast-on-draw chain (R58), fatigue on an empty
// deck, the hand cap burning the overflow (R4, R317), a draw limit stopping the rest (R457).
//
// "Then discard": R641 makes it random, read during resolution over the hand the draws left. The hand
// is read as the list reaches the discard, not as the card is played — a draw that pauses (a
// cast-on-draw card's own prompt) resumes into the rest of the list, and the discard then reads the
// hand as it stands.
//
// "Fewer in hand, all of them": with no more cards in hand than the discard asks for the whole hand
// goes, in hand order and drawing no rng. That half reads the hand lazily (`forEachCard`, as the
// list reaches it); with more cards than that, the first half finds nothing to do and the random
// discard takes exactly that many.
//
// The two numbers are the declared `draw` and `discard` (R386), read through `param`; the Radiant
// face differs only in its `draw`, so both faces run this one script.

import { param, zoneCards, zoneCount, type EffectContext, type Script } from "@jackioh/engine";
import { discard, discardRandom, draw, forEachCard } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-026");

/** The whole hand, when it holds no more cards than the discard asks for; otherwise nothing. */
function wholeHandIfNoChoice(ctx: EffectContext): readonly string[] {
  if (zoneCount(ctx.state, ctx.controller, "hand") > param(ctx, "discard")) return [];
  return zoneCards(ctx.state, ctx.controller, "hand").map((card) => card.id);
}

export const base: Script = {
  cry: (ctx) => [
    draw({ count: param(ctx, "draw") }),
    forEachCard({ cards: wholeHandIfNoChoice, each: (instanceId) => discard({ target: { of: "instance", instanceId } }) }),
    discardRandom({ count: param(ctx, "discard") }),
  ],
};

// The same script: the Radiant face's 5 draws are its declared `draw`, which `param` reads off the running face.
export const radiant: Script = base;
