// C+ #75 J-lease J-Jungle EX-plorer (SPEC §8.7 row 75). (2) Unit, Legendary, 5/5 → 10/10.
//   Base:    "Cry: Shuffle {packs|J-lease J-Jungle EX-plorer Pack|J-lease J-Jungle EX-plorer Packs} into
//            your deck."
//   Radiant: "Cry: Shuffle {packs|Radiant J-lease J-Jungle EX-plorer Pack|…} into your deck."
//   Engine:  "The Pack is C+ #75.1, shuffled in openly at a random position (§6.3), so its owner's list
//            shows it (R311); R80's cap turns it away. The Radiant Pack's five cards cost (0), so the
//            effect grows with the stats (R275). Tunes: packs 1 ↑."
//
// `shuffleInto` is §6.3's Shuffle into a library: a fresh Pack at a random position (one rng draw each),
// shown to the deck's owner (R311) and refused at the library cap (R80). A copy of this Unit is summoned
// with no Cry (R1), so only a played one shuffles.

import { param, type Script } from "@jackioh/engine";
import { shuffleInto } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-075");

/** §7, §8.7: the token the Cry shuffles in. */
const PACK = cardDef("classicplus-075-1").id;

function explorer(radiant: boolean): Script {
  return {
    cry: (ctx) => [shuffleInto({ defId: PACK, count: param(ctx, "packs"), ...(radiant ? { radiant: true } : {}) })],
  };
}

export const base: Script = explorer(false);

export const radiant: Script = explorer(true);
