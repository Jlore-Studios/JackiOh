// C+ #65.1 Rotten Grape (SPEC §8.7 row 65.1). (1) Spell, Fruit, Token (printed Common).
//   Base:    "Your hero loses 5 health."
//   Radiant: "Your hero loses 1 health."
//   Engine:  "Lose health (§6.3, R18): no pipeline, no Armor, no cap. The Radiant's smaller loss is its
//            upgrade (R275). A Fruit for every Fruit rule (C+ #64, C+ #68). Tunes: none."
//
// R18: losing health is not damage — `loseHealth` takes it straight off the hero, past Armor, under no
// per-hit cap and through no replacement window, and a hero at 0 loses at the state check after the
// list (§4.5). The entry declares no params (the Rotten Grape's numbers are printed), so the two
// amounts are named here.

import type { Script } from "@jackioh/engine";
import { loseHealth } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-065-1");

/** §8.7: the health the base face costs its own hero. */
const BASE_LOSS = 5;
/** §8.7: the Radiant face's smaller loss, its upgrade (R275). */
const RADIANT_LOSS = 1;

export const base: Script = { cry: () => [loseHealth({ player: "self", amount: BASE_LOSS })] };

export const radiant: Script = { cry: () => [loseHealth({ player: "self", amount: RADIANT_LOSS })] };
