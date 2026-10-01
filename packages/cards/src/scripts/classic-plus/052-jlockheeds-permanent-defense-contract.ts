// C+ #52 Jlockheed's Permanent Defense Contract (SPEC §8.7 row 52). (2) Spell, Jlockeed, Epic.
//   Base:    "For the rest of the game: At the start of your turn, add {cards|random Jlockheed card|…}
//            to your hand." — cards 1
//   Radiant: "… add {cards|random Radiant Jlockheed card|…} to your hand. Each costs ({discount}) less."
//            — cards 1, discount 1
//   Engine:  "A player effect for the rest of the game (§10.1): a `never`-expiry player modifier on the
//            caster that acts at their start of turn, among the start-of-turn triggers (R62). The pool
//            is the non-token `Jlockeed` cards but this one (R387): #13, #14, C+ #48, C+ #51. Several
//            contracts stack, one card each. Radiant: `costMod` −1 on the card. The hand cap burns
//            extras (§2.4). Tunes: cards 1 ↑; Radiant discount 1 ↑."
//
// `forRestOfGame` (B5 E28, R458) re-enters `delayed` at each of the caster's turn starts with no `self`
// (R127), so the numbers are read through `param` as the Spell resolves and carried in the entry's
// data (R594). The pool leaves this card out by its def id, which the re-entry carries (R387); the price lands
// only on a card that reached the hand (§2.4, R4).

import { param, type EffectContext, type Script } from "@jackioh/engine";
import { addRandomFromCatalog, forRestOfGame } from "@jackioh/engine/effects";
import { fillParams } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-052");

/** §10.6's data is JSON: a number carried in the entry, or the printed value when it is missing. */
function carried(ctx: EffectContext, key: "cards" | "discount"): number {
  const value = ctx.data[key];
  return typeof value === "number" ? value : param(ctx, key);
}

function contract(radiant: boolean): Script {
  return {
    cry: (ctx) => {
      const numbers = { cards: param(ctx, "cards"), discount: param(ctx, "discount") };
      const label = fillParams(def, radiant ? "radiant" : "base", numbers);
      return [forRestOfGame({ step: "contract", label, data: numbers })];
    },
    delayed: (ctx) => [
      addRandomFromCatalog({
        query: { tags: ["Jlockeed"] },
        count: carried(ctx, "cards"),
        ...(radiant ? { radiant: true, costMod: -carried(ctx, "discount") } : {}),
      }),
    ],
  };
}

export const base: Script = contract(false);

export const radiant: Script = contract(true);
