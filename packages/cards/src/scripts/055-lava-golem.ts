// #55 Lava Golem (SPEC §8.3, §6.3 Tribute/Sacrifice, §3.2; R4, R11, R65, R81, R90, R101, R360).
// Unit 10/5 → 20/10, cost 3, Rare.
//   Base:    "Taunt, Tribute 3. Can use opposing Units as Tributes. If opposing Units are used,
//            summon for your opponent."
//   Radiant: "Taunt, Tribute 3. Can use opposing Units as Tributes."
// Patch v0.1.1 took Armor 3 off both faces and Indestructible off the Radiant one, and gave the
// base face its price: a Tribute that takes any opposing unit summons the Golem for the opponent.
//
// KEYWORDS ARE DATA, NOT SCRIPT. Both faces print [Taunt], read straight off the def by §10.4
// layer 1 (`faceOf` in engine/src/layers.ts), so this file grants nothing.
//
// THE COST IS THE SCRIPT. §6.3 calls Tribute "an additional cost of playing a card", so it lives in
// the play validator (`playChoices.ts`), and the units chosen travel in the `play` action's own
// `tributes` list (R81). What this file declares, and who reads it:
//   * `tribute` — `tributeCostOf(card)`: Tribute 3, with the Sheep Token worth 2 (`tributeValueOf`,
//     §3.2), and `refuseTributes` refusing a board that cannot pay; the units are sacrificed at §10.5
//     step 2, which bypasses Indestructible and counts as a death (§6.3).
//   * `tributeEnemies` — "Can use opposing Units as Tributes" (R101): `legalTributeUnits` offers both
//     sides' units only to a card that says so.
//   * `enemyTributeHandsOver` — the base face's "If opposing Units are used, summon for your
//     opponent" (R360): step 2 records whether the Tribute it paid took an opposing unit, and step 4
//     then puts the Golem in the opponent's zone in the lane the player named, else their leftmost
//     open one (R15), under their control; it stays the player's card and the player's play.
//
// R65/§6.3: a Tribute is an *additional* cost, so a mana price of 0 does not touch it — #41
// Sheepish's radiant "Add a Lava Golem to your hand. It costs (0)." is a `costOverride` of 0 on the
// mana term alone and that free copy still needs three units on the field.
//
// Nothing here is a hook: a play cost, and where the play lands, have to be readable before the card
// resolves.

import type { Script, StaticFlags } from "@jackioh/engine";
import { cardDef } from "../catalog-data";

export const def = cardDef("core-055");

/** §8: "Tribute 3", counted with the Sheep Token worth 2 (§3.2). */
const TRIBUTE_COST = 3;

/** The Radiant face: its Tribute, and the permission to pay it with opposing units (R101). */
const RADIANT_FLAGS: StaticFlags = {
  tribute: TRIBUTE_COST,
  tributeEnemies: true,
};

/** The base face adds its price: paid with an opposing unit, it is summoned for the opponent (R360). */
const BASE_FLAGS: StaticFlags = {
  ...RADIANT_FLAGS,
  enemyTributeHandsOver: true,
};

export const base: Script = { staticFlags: BASE_FLAGS };

export const radiant: Script = { staticFlags: RADIANT_FLAGS };
