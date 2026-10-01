// C+ #19.1 Top Loser (SPEC §8.7 row 19.1): Armor 3 (Radiant: 6, Immune to Spells, catalog keywords);
// never in Defense Position, and only a Unit in its lane may attack it (E35 static flags).

import type { Script } from "@jackioh/engine";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-019-1");

export const base: Script = {
  staticFlags: { attackedOnlyFromLane: true, neverDefense: true },
};

// The same script: Armor 6 and Immune to Spells are the Radiant face's catalog keywords.
export const radiant: Script = base;
