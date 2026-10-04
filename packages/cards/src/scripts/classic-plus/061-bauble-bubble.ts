// C+ #61 Bauble Bubble (SPEC §8.7 row 61). (1) Field Spell, Fruit, Rare.
//   Base:    "Death: Add {cards|Stockpile|Stockpiles} to your hand. Each costs (0)." — cards 2
//   Radiant: "Death: Add {cards|Radiant Stockpile|Radiant Stockpiles} to your hand. Each costs (0)."
//   Engine:  "A Death that fires from either zone (§4.5): it fires when the card goes from the field to a
//            graveyard, destroyed or sacrificed, as a Unit's does; not on bounce, exile, transform or
//            steal (§6.2). Stockpile is #5; `costOverride` 0; the hand cap burns extras. A bait card:
//            nothing until it pops. Tunes: cards 2 ↑."
//
// §4.5 step 3 fires a collected backrow card's Death as it does a Unit's (the engine's, proved in
// `packages/engine/test/backrow-death.test.ts`), so the card is one Death hook.

import { param, type Script } from "@jackioh/engine";
import { addToHand } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-061");

const STOCKPILE = cardDef("core-005").id;

function baubleBubble(radiant: boolean): Script {
  return {
    death: (ctx) =>
      Array.from({ length: param(ctx, "cards") }, () => addToHand({ defId: STOCKPILE, costOverride: 0, radiant })),
  };
}

export const base: Script = baubleBubble(false);

export const radiant: Script = baubleBubble(true);
