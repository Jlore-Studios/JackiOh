// C+ #4 Juhan Biggest Bat (SPEC §8.7 row 4). (3) Unit, CN, Common, 9/6 → 18/12, Stack (+ First Strike).
// Played onto a pile, every dormant card beneath becomes a copy of this on its face (E24): owner and
// controller kept, still dormant, no Cry (R1), Immutable ones untouched (R23). Onto an empty zone it
// does nothing. Its Cry is the arrival: only a play fires it (R1).

import type { Script } from "@jackioh/engine";
import { transformBeneath } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-004");

export const base: Script = { cry: () => [transformBeneath()] };

/** The Radiant face adds First Strike and doubles the stats (catalog data); its copies are Radiant as it is. */
export const radiant: Script = base;
