// #45 Deft Duelist (SPEC §8.2). Unit 4/3 → 8/6, Human, cost 2, Rare.
//   Base:    "Charge, Deft"
//   Radiant: "Charge, Armor 1, Deft" — §8 Conventions: a keyword cell without "Plus" gives the
//            radiant face's COMPLETE keyword list.
//
// Charge, Armor 1 and Deft are PRINTED keywords: `catalog.json` carries them on `base.keywords`
// and `radiant.keywords`, and §10.4 layer 1 reads them off the def. Granting any of them here
// would be a second source — for Armor it would literally double it, because §10.4 sums Armor
// across sources. So both scripts are empty.
//
// R49 (two exertions: one attack plus one switch in a turn) is the Deft keyword, which
// `combat.ts` reads through §10.4's layers, so a granted Deft works the same as the printed one.

import type { Script } from "@jackioh/engine";
import { cardDef } from "../catalog-data";

export const def = cardDef("core-045");

export const base: Script = {};

export const radiant: Script = base;
