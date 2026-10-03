// C #73 Nurse Cleaver (SPEC §8.6 row 73). (2) Unit, Common, 3/6 → 6/12.
//   Base:    "Rush, Cleave, Lifesteal"
//   Radiant: "Charge, Cleave, Lifesteal"
//   Engine:  "Keywords only. Tunes: none."
//
// Nothing to script: the stats and both faces' keywords are the catalog's, and the engine reads them.
//   Rush      — §4.1: it may attack a Unit the turn it enters, never the hero (Charge: the hero too).
//   Cleave    — §4.4 step 10: each hit of its attack also hits the defender's neighbours (§3.1), as
//               separate damage instances; combat only, and part of the attack (R63).
//   Lifesteal — §4.4 step 8: its controller's hero heals the amount each hit deals, the Cleave hits
//               included.
// Its proof: `test/classic/073-nurse-cleaver.test.ts`.

import type { Script } from "@jackioh/engine";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-073");

export const base: Script = {};

// The same empty script: the Radiant face's 6/12 and its Charge for Rush are catalog data.
export const radiant: Script = base;
