// C+ #6 Wrong-House Attacker (SPEC §8.7 row 6). (1) Unit, Human, Common, 1/1 → 2/2.
// Rush, Lifesteal, Poisonous (Radiant: plus Reborn): keywords only, printed in the catalog and applied
// by the layers (§10.4), so neither face has anything to run.

import type { Script } from "@jackioh/engine";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-006");

export const base: Script = {};

/** The Radiant face adds Reborn and doubles the stats: catalog data only. */
export const radiant: Script = base;
