// C #80 BOOM! Big Max (SPEC §8.6 row 80). (4) Unit, Legendary, 13/8 → 26/16.
//   Base:    "Tribute 3, Rush, Trample, Indestructible" (balance patch 1: 13/8, Tribute 3)
//   Radiant: "Tribute 3, Charge, Trample, Indestructible"
//   Engine:  "Keywords only. Tribute 3 (§6.3, R101) may pay for its own zone (R391, §3.2);
//            Indestructible gives no Taunt (R347). Balance patch 1 halves the base attack to 13, so the
//            Radiant 26/16 doubles it exactly (R275), and the Radiant face trades Rush for Charge.
//            Tunes: none."
//
// The one thing the script carries is the Tribute cost, §6.3's `staticFlags.tribute` (as #66 The Rock
// carries its own): the play validator (`playChoices`) refuses the play when the board cannot pay 3
// (a Sheep Token is worth 2, R101, so it pays with one more Unit) and pairs each zone with the paying sets that leave it open,
// so a full row's one-card pile it tributes is its zone (R391). The keywords are the catalog's:
//   Rush / Charge   — §4.1: Rush attacks Units the turn it enters; the Radiant's Charge the hero too.
//   Trample         — §4.4 step 9, R63: the excess over the defender's health hits that side's hero.
//   Indestructible  — §4.4 step 4 (it takes no damage), §4.5 and R46 (a destroy knocks it into Attack
//                     Position), R347 (never Taunt), R69 (it still dies if its max health reaches 0).
// Its proof: `test/classic/080-boom-big-max.test.ts`.

import type { Script } from "@jackioh/engine";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-080");

/** §6.3 "Tribute 3": three of your Units, or a Sheep Token (worth 2, §3.2) and one more. */
const TRIBUTE_COST = 3;

export const base: Script = { staticFlags: { tribute: TRIBUTE_COST } };

// The same script: the Radiant face's 16 health and Charge are catalog data, and it keeps Tribute 3.
export const radiant: Script = base;
