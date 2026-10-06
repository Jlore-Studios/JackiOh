// C+ #38 Solarius (SPEC §8.7 row 38): (2) Unit, Epic, 3/2 → 6/4.
//   Base:    "Spell Damage +2. Death: Shuffle a Solarius-Prime into your deck." (no Cry, balance patch 1)
//   Radiant: "Spell Damage +5. Death: Shuffle a Radiant Solarius-Prime into your deck."
// Spell Damage is the catalog's numbered keyword, which §4.4 step 0 (`damage.ts`) reads off the field;
// the Death shuffles a fresh C+ #38.1 in at a random position, R80's cap turning it away.

import type { Script } from "@jackioh/engine";
import { shuffleInto } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-038");

const SOLARIUS_PRIME = "classicplus-038-1";

function solarius(radiant: boolean): Script {
  return {
    death: () => [shuffleInto({ defId: SOLARIUS_PRIME, count: 1, radiant })],
  };
}

export const base: Script = solarius(false);

export const radiant: Script = solarius(true);
