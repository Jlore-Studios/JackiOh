// C+ #11 Anime Armor (SPEC §8.7 row 11). (2) Unit, Rare, 4/4 → 8/8 (Radiant: Reborn).
// Aura: while it acts on the field its controller's hero takes at most {cap} from each damage instance
// (§4.4 step 3, E6's per-hit cap: the lowest cap wins beside Anti-oneshot Armor's). Lose health (R18)
// and Set health are not damage and are not capped.

import { param, type Script } from "@jackioh/engine";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-011");

export const base: Script = { heroGuard: (args) => [{ cap: param(args, "cap") }] };

/** The Radiant face adds Reborn and doubles the stats: catalog data only. */
export const radiant: Script = base;
