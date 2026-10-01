// C+ #66 Vine of Grapes (SPEC §8.7 row 66). (3) Spell, Fruit, Rare.
//   Base:    "Add {grapes|Grape|Grapes} to your hand, each rolled: Rotten Grape 12%, Normal Grape 60%,
//            Large Grape 20%, Golden Grape 7%, Mythic Grape 1%."
//   Radiant: "Lucky 1 / Add {grapes|Radiant Grape|Radiant Grapes} to your hand, each rolled: …"
//   Engine:  "As C+ #65: five independent rolls (`GRAPE_ODDS`), the Radiant's with Lucky 1; the hand cap
//            burns extras. Tunes: grapes 5 ↑."
//
// C+ #65 Two Grapes with five: the same engine verb (`addRolledGrapes`, effects/fruit.ts), the count
// its own declared number. The designer's Radiant said 3, read as copied from Two Grapes; it is 5 so
// the Radiant face never loses two Grapes (SPEC §8.7 row 66).

import { param, type Script } from "@jackioh/engine";
import { addRolledGrapes } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-066");

function vineOfGrapes(radiant: boolean): Script {
  return {
    cry: (ctx) => [addRolledGrapes({ count: param(ctx, "grapes"), ...(radiant ? { radiant: true } : {}) })],
  };
}

export const base: Script = vineOfGrapes(false);

export const radiant: Script = vineOfGrapes(true);
