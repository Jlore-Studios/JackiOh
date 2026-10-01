// C+ #65 Two Grapes (SPEC §8.7 row 65). (1) Spell, Fruit, Rare.
//   Base:    "Add {grapes|Grape|Grapes} to your hand, each rolled: Rotten Grape 12%, Normal Grape 60%,
//            Large Grape 20%, Golden Grape 7%, Mythic Grape 1%."
//   Radiant: "Lucky 1 / Add {grapes|Radiant Grape|Radiant Grapes} to your hand, each rolled: …"
//   Engine:  "Three independent weighted rolls (`GRAPE_ODDS` in `config.ts`) over C+ #65.1 to C+ #65.5,
//            a pool the text names; Lucky 1 (§6.1) rolls twice and keeps the better, in the order
//            Rotten < Normal < Large < Golden < Mythic. The hand cap burns extras (§2.4). Three Grapes
//            from 'Two Grapes' is the designer's joke, kept. Tunes: grapes 3 ↑."
//
// The roll is the engine's `addRolledGrapes` (effects/fruit.ts, shared with C+ #66 Vine of Grapes):
// one draw over `GRAPE_ODDS` per Grape, Lucky's extra draws keeping the later entry, each Grape added
// before the next is rolled. The Lucky it reads is the running card's own — the Radiant face prints
// Lucky 1, and a Degrade or an Upgrade of that number moves it (B3.4's X change) — so both faces run
// one hook and differ by the face's keyword and by whether the Grapes are Radiant.

import { param, type Script } from "@jackioh/engine";
import { addRolledGrapes } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-065");

function twoGrapes(radiant: boolean): Script {
  return {
    cry: (ctx) => [addRolledGrapes({ count: param(ctx, "grapes"), ...(radiant ? { radiant: true } : {}) })],
  };
}

export const base: Script = twoGrapes(false);

export const radiant: Script = twoGrapes(true);
