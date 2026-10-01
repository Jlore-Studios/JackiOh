// C+ #67 Pear (SPEC §8.7 row 67). (2) Spell, Fruit, Rare.
//   Base:    "Summon {units|random (1) Cost Common Unit|random (1) Cost Common Units}."
//   Radiant: "Summon {units|random Radiant (1) Cost Common Unit|random Radiant (1) Cost Common Units}."
//   Engine:  "Non-token Units printed at (1) Cost (R65) with rarity Common, every set (R380); repeats
//            allowed (R60); summoned, so no Cry (R1), placed per R64; a full board takes fewer.
//            Tunes: units 2 ↑."
//
// Each Unit is its own `summonRandom`: one pick of the pool per summon (R60's repeats), placed in the
// leftmost open, unlocked, unreserved zone (R64), and no pick at all when no zone is open (R129), so a
// nearly full board takes one and a full one none. The pool's cost is R65's out-of-play cost, which is
// what `cost` in a catalog query reads (an X-cost card reads 0, never 1).

import { param, type Script } from "@jackioh/engine";
import { summonRandom } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-067");

/** §8.7: the printed cost of the Units it summons. */
const UNIT_COST = 1;

function pear(radiant: boolean): Script {
  return {
    cry: (ctx) =>
      Array.from({ length: param(ctx, "units") }, () =>
        summonRandom({
          query: { type: "Unit", cost: UNIT_COST, rarity: "Common" },
          ...(radiant ? { radiant: true } : {}),
        }),
      ),
  };
}

export const base: Script = pear(false);

export const radiant: Script = pear(true);
