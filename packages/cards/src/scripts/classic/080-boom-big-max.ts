// C #80 BOOM! Big Max (SPEC §8.6 row 80). (4) Unit, Legendary, 26/8 → 26/16.
//   Base:    "Tribute 2, Rush, Trample, Indestructible"
//   Radiant: "Tribute 2, Charge, Trample, Indestructible"
//   Engine:  "Keywords only. Tribute 2 (§6.3, R101) may pay for its own zone (R391, §3.2);
//            Indestructible gives no Taunt (R347). The Radiant attack stays 26 by the designer's
//            number, a named exception in the Radiant-standard test (R275), and the Radiant face trades
//            Rush for Charge to meet the keyword half. Tunes: none."
//
// The one thing the script carries is the Tribute cost, §6.3's `staticFlags.tribute` (as #66 The Rock
// carries its own): the play validator (`playChoices`) refuses the play when the board cannot pay 2
// (a Sheep Token alone pays, worth 2, R101) and pairs each zone with the paying sets that leave it open,
// so a full row's one-card pile it tributes is its zone (R391). The keywords are the catalog's:
//   Rush / Charge   — §4.1: Rush attacks Units the turn it enters; the Radiant's Charge the hero too.
//   Trample         — §4.4 step 9, R63: the excess over the defender's health hits that side's hero.
//   Indestructible  — §4.4 step 4 (it takes no damage), §4.5 and R46 (a destroy knocks it into Attack
//                     Position), R347 (never Taunt), R69 (it still dies if its max health reaches 0).
// Its proof: `test/classic/080-boom-big-max.test.ts`.

import type { Script } from "@jackioh/engine";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-080");

/** §6.3 "Tribute 2": two of your Units, or one Sheep Token, worth 2 (§3.2). */
const TRIBUTE_COST = 2;

export const base: Script = { staticFlags: { tribute: TRIBUTE_COST } };

// The same script: the Radiant face's 16 health and Charge are catalog data, and it keeps Tribute 2.
export const radiant: Script = base;
