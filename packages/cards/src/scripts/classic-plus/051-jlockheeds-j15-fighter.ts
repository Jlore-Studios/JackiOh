// C+ #51 Jlockheed's J15 Fighter (SPEC §8.7 row 51). (3) Unit, Jlockeed, Epic, 7/2 → 14/4.
//   Base:    "First Strike, Rush. Can't be in Defense Position. Can't be attacked."
//   Radiant: "First Strike, Rush, Divine Shield" and the same.
//   Engine:  "Can't be attacked (§6.1, §4.2 step 2): no attack may name it, a forced one included (a
//            forced attack on it does not happen, and "a random enemy" never draws it); it is still
//            targeted by effects and hit by "all" effects. A Taunt it gains binds nobody, since it
//            cannot be attacked. The Defense restriction is a position validator flag. Tunes: none."
//
// Both restrictions are static flags the engine reads everywhere (`restrictions.cannotBeAttacked`,
// #65.1's `neverDefense`); the keywords are the catalog's. Nothing else to do.

import type { Script } from "@jackioh/engine";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-051");

export const base: Script = { staticFlags: { neverDefense: true, cantBeAttacked: true } };

// The same script: the Radiant face differs only in its stats and Divine Shield, both catalog data.
export const radiant: Script = base;
