// C+ #7 The House (SPEC §8.7 row 7, R406). (3) Field Spell, Rare.
// Cry and each start of its controller's turn: summon a Right-house defender (Core #3) with chance
// 2 in 3, else a Wrong-House Attacker (C+ #6); Radiant: one of each. Fresh base-face cards, yours, no
// Cry (R1), R64's placement; a full board summons nothing and rolls nothing (R129).

import { firstFreeZone, type Hook, type Script } from "@jackioh/engine";
import { summon } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-007");

/** R406: "Right-House Protector" is Core #3, the Wrong-House Attacker's twin. */
const RIGHT_HOUSE = "core-003";
const WRONG_HOUSE = "classicplus-006";
/** R406: "(2 in 3)" — the odds of the Right-house defender. */
const RIGHT_HOUSE_ODDS = 2 / 3;

const summonOne: Hook = (ctx) =>
  firstFreeZone(ctx.state, ctx.controller, "units") === null
    ? []
    : [summon({ defId: ctx.rng.chance(RIGHT_HOUSE_ODDS) ? RIGHT_HOUSE : WRONG_HOUSE })];

const summonBoth: Hook = () => [summon({ defId: RIGHT_HOUSE }), summon({ defId: WRONG_HOUSE })];

export const base: Script = { cry: summonOne, startOfTurn: summonOne };

export const radiant: Script = { cry: summonBoth, startOfTurn: summonBoth };
