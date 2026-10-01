// C #79 Risky Die (SPEC §8.6 row 79, BUILD M9 Classic row C 79). (1) Spell, Common.
//   Base:    "Draw {draw}. They cost (1) less. Then exile each of them that costs more than
//            ({threshold})." (3, 0)
//   Radiant: the same at ({threshold}) 1 — "then exile each of them that costs (2) or more".
//   Engine:  "The cards the three draws put in your hand (a card cast on draw never gets there, R58; a
//            burned one isn't there): `costMod` −1 each, then exile those whose cost in hand (R65) is
//            above 0 (Radiant: above 1); an X-cost card counts 0 in hand (R65) and is kept. Tunes: draw
//            3 ↑; kept threshold 0 ↑."
//
// Readings:
//   - "Draw N" is N draws (§2.4): cast on draw, fatigue, the hand cap and a draw limit each act on
//     their own draw, and a cast on draw that asks pauses the rest (R113).
//   - "They" are the cards the draws moved from the library into the hand: the library is noted on
//     the resolving card as the Spell begins (`remember`, so a pause cannot lose it), and "they" are
//     then the hand cards that were in it. A card cast on draw, a burned card, a draw a limit stopped
//     and a fatigue hit put nothing there; a card an effect created in the hand meanwhile (a cast on
//     draw that adds cards) was never in the library; and cards already in the hand are untouched.
//   - Each of them gets `costMod` −1, which persists in every zone (R78); then each whose cost in hand
//     now (R65's `effectiveCost`: a player's discounts and surcharges included, an X-cost card 0) is
//     more than the threshold is exiled, publicly. The kept ones stay hidden in hand (R97).
// Both numbers are declared and read through `param` (R386): the draw count, and the kept threshold
// ("↑": an Upgrade keeps more).

import type { CardInstance, Effect, EffectContext, Script } from "@jackioh/engine";
import { effectiveCost, param, recalled, zoneCards } from "@jackioh/engine";
import { draw, exile, forEachCard, remember, setCostMod } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-079");

/** Where the resolving card notes its controller's library as the Spell begins. */
const LIBRARY_KEY = "riskyDieLibrary";

/** "They": the cards in its controller's hand now that were in their library as the Spell began. */
function drawnIntoHand(ctx: EffectContext): CardInstance[] {
  const noted = recalled(ctx, LIBRARY_KEY);
  const library = new Set(Array.isArray(noted) ? noted.filter((id): id is string => typeof id === "string") : []);
  return zoneCards(ctx.state, ctx.controller, "hand").filter((card) => library.has(card.id));
}

/** "They cost (1) less." */
const RISKY_DISCOUNT = -1;

const cry = (ctx: EffectContext): Effect[] => [
  remember({ key: LIBRARY_KEY, value: zoneCards(ctx.state, ctx.controller, "library").map((card) => card.id) }),
  draw({ count: param(ctx, "draw") }),
  forEachCard({
    cards: drawnIntoHand,
    each: (instanceId) => setCostMod({ target: { of: "instance", instanceId }, amount: RISKY_DISCOUNT }),
  }),
  forEachCard({
    cards: (now) => drawnIntoHand(now).filter((card) => effectiveCost(now.state, card) > param(now, "threshold")),
    each: (instanceId) => exile({ target: { of: "instance", instanceId } }),
  }),
];

export const base: Script = { cry };

// The same script: the Radiant face differs only in its declared threshold (1), read through `param`.
export const radiant: Script = base;
