// C #30 Recycle (SPEC §8.6 row 30). (1) Spell, Rare.
//   Base:    "Shuffle your graveyard into your deck. Draw {draw}." — draw 1
//   Radiant: "Shuffle your graveyard into your deck. They cost ({discount}) less. Draw {draw}." — 1, 1
//   Engine:  "Every card in your graveyard (this Spell is resolving, not in it) at random positions;
//            R80's cap: cards that don't fit stay in the graveyard, each reported by
//            `libraryOverflow` (R316). Radiant: `costMod` −1 on each shuffled card. Then the draw. The
//            name sits inside C #64 Malzahar's Recycler and is a rules word the reference proof never
//            reads as this card unless `refs` lists it (R381). Tunes: draw 1 ↑; Radiant discount 1 ↑."
//
// The graveyard is read once, as the Spell begins to resolve — it is in the resolving zone then
// (§10.5), so it is not among the cards — and each card is shuffled in, in graveyard order, each at
// its own random position (`shuffleCardInto`): the same card, its `costMod` and the rest of what R78
// keeps, going in openly to its owner (R311) at a slot neither player reads (R97). A card a full
// library turns away stays where it is (R80, R316).
//
// Radiant: "They" are the cards that went in, so the discount lands on each of them that is now in
// the deck, and on no card the cap left behind: a `costMod` of −{discount}, which R78 keeps in every
// zone. A change made inside a library is read by nobody (R177), so its `costChanged` stays hidden.
//
// Then the draw, §2.4's pipeline. An empty graveyard only draws.
//
// The numbers are the declared `draw` and `discount` (R386), read through `param`.

import { param, zoneCards, type Effect, type EffectContext, type Script } from "@jackioh/engine";
import { draw, forEachCard, setCostMod, shuffleCardInto } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-030");

function recycle(discounts: boolean): Script {
  return {
    cry: (ctx): Effect[] => {
      // The graveyard as the Spell begins to resolve, read once for both halves.
      const shuffled = zoneCards(ctx.state, ctx.controller, "graveyard").map((card) => card.id);
      const wentIn = (c: EffectContext): string[] =>
        zoneCards(c.state, c.controller, "library")
          .filter((card) => shuffled.includes(card.id))
          .map((card) => card.id);
      return [
        forEachCard({ cards: () => shuffled, each: (instanceId) => shuffleCardInto({ instanceId }) }),
        ...(discounts
          ? [
              forEachCard({
                cards: wentIn,
                each: (instanceId) => setCostMod({ target: { of: "instance", instanceId }, amount: -param(ctx, "discount") }),
              }),
            ]
          : []),
        draw({ count: param(ctx, "draw") }),
      ];
    },
  };
}

export const base: Script = recycle(false);

export const radiant: Script = recycle(true);
