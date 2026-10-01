// C+ #65.5 Mythic Grape (SPEC §8.7 row 65.5). (0) Spell, Fruit, Token (printed Mythic).
//   Base:    "Replace your hand with random Mythic cards. They cost (0)."
//   Radiant: "Replace your hand with random Radiant Mythic cards. They cost (0)."
//   Engine:  "Each other card in your hand goes to your graveyard (a unit-token card ceases to exist,
//            R11) and is replaced one for one by a random non-token Mythic of every set (R380), #76 Field
//            of Dreams' reading; repeats allowed (R60); `costOverride` 0. An empty hand gets nothing.
//            Tunes: none."
//
// The Grape is resolving, not in the hand, so "each other card" is the whole hand as it resolves.
// `replaceHandWithRandom` (effects/fruit.ts) MOVES each card to the graveyard — it is not a discard,
// so nothing that answers a discard sees it (BUILD M9) — and then makes as many random Mythics, each a
// fresh card through Add to hand. The pool is the non-token cards of rarity Mythic, every set: a token's
// printed rarity never feeds a pool (§5), so no Grape is ever one of them, this one included (R387).
// An empty hand moves nothing and draws nothing from the rng (R129).

import type { Script } from "@jackioh/engine";
import { replaceHandWithRandom } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-065-5");

/** §8.7: "They cost (0)". */
const SET_COST = 0;

function mythicGrape(radiant: boolean): Script {
  return {
    cry: () => [
      replaceHandWithRandom({ query: { rarity: "Mythic" }, costOverride: SET_COST, ...(radiant ? { radiant: true } : {}) }),
    ],
  };
}

export const base: Script = mythicGrape(false);

export const radiant: Script = mythicGrape(true);
